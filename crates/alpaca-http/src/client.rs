use std::sync::Arc;
use std::time::{Duration, Instant};

use alpaca_core::BaseUrl;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde::de::DeserializeOwned;

use crate::auth::Authenticator;
use crate::meta::{ErrorMeta, HttpResponse, ResponseMeta};
use crate::observer::{
    ErrorEvent, NoopObserver, RequestStart, ResponseEvent, RetryEvent, TransportObserver,
};
use crate::rate_limit::ConcurrencyLimit;
use crate::request::{NoContent, RequestBody, RequestParts};
use crate::retry::{RetryConfig, RetryDecision};
use crate::Error;

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    default_headers: HeaderMap,
    request_id_header_name: HeaderName,
    retry_config: RetryConfig,
    observer: Arc<dyn TransportObserver>,
    concurrency_limit: ConcurrencyLimit,
}

#[derive(Clone)]
pub struct HttpClientBuilder {
    reqwest_client: Option<reqwest::Client>,
    timeout: Duration,
    default_headers: HeaderMap,
    request_id_header_name: HeaderName,
    retry_config: RetryConfig,
    observer: Arc<dyn TransportObserver>,
    concurrency_limit: ConcurrencyLimit,
}

struct ResponseParts {
    meta: ResponseMeta,
    body: String,
}

impl HttpClient {
    #[must_use]
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    pub async fn send_json<T>(
        &self,
        base_url: &BaseUrl,
        request: RequestParts,
        authenticator: Option<&dyn Authenticator>,
    ) -> Result<HttpResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        let response = self.send(base_url, &request, authenticator).await?;
        let parsed = serde_json::from_str(&response.body).map_err(|error| {
            let meta = ErrorMeta::from_response_meta(response.meta.clone(), response.body.clone());
            let error = Error::Deserialize {
                message: error.to_string(),
                meta: Some(meta.clone()),
            };
            self.observer.on_error(&ErrorEvent { meta: Some(meta) });
            error
        })?;

        self.observer.on_response(&ResponseEvent {
            meta: response.meta.clone(),
        });
        Ok(HttpResponse::new(parsed, response.meta))
    }

    pub async fn send_text(
        &self,
        base_url: &BaseUrl,
        request: RequestParts,
        authenticator: Option<&dyn Authenticator>,
    ) -> Result<HttpResponse<String>, Error> {
        let response = self.send(base_url, &request, authenticator).await?;
        self.observer.on_response(&ResponseEvent {
            meta: response.meta.clone(),
        });
        Ok(HttpResponse::new(response.body, response.meta))
    }

    pub async fn send_no_content(
        &self,
        base_url: &BaseUrl,
        request: RequestParts,
        authenticator: Option<&dyn Authenticator>,
    ) -> Result<HttpResponse<NoContent>, Error> {
        let response = self.send(base_url, &request, authenticator).await?;
        if response.meta.status() != 204 {
            let meta = ErrorMeta::from_response_meta(response.meta, response.body);
            let error = Error::HttpStatus(meta.clone());
            self.observer.on_error(&ErrorEvent { meta: Some(meta) });
            return Err(error);
        }

        self.observer.on_response(&ResponseEvent {
            meta: response.meta.clone(),
        });
        Ok(HttpResponse::new(NoContent, response.meta))
    }

    async fn send(
        &self,
        base_url: &BaseUrl,
        request: &RequestParts,
        authenticator: Option<&dyn Authenticator>,
    ) -> Result<ResponseParts, Error> {
        let _permit = self.concurrency_limit.acquire().await?;
        let url = base_url.join_path(request.path());
        let mut attempt = 0;
        let started_at = Instant::now();

        loop {
            self.observer.on_request_start(&RequestStart {
                operation: request.operation().map(ToOwned::to_owned),
                method: request.method(),
                url: url.clone(),
            });

            let request_builder = self.build_request(&url, request, authenticator)?;
            let response = match request_builder.send().await {
                Ok(response) => response,
                Err(error) => {
                    match self.retry_config.classify_transport_error(
                        &request.method(),
                        attempt,
                        started_at.elapsed(),
                    ) {
                        RetryDecision::RetryAfter(wait) => {
                            self.observer.on_retry(&RetryEvent {
                                operation: request.operation().map(ToOwned::to_owned),
                                method: request.method(),
                                url: url.clone(),
                                attempt: attempt + 1,
                                status: None,
                                wait,
                            });
                            tokio::time::sleep(wait).await;
                            attempt += 1;
                            continue;
                        }
                        RetryDecision::DoNotRetry => {
                            let error = Error::from_reqwest(error, None);
                            self.observer.on_error(&ErrorEvent { meta: None });
                            return Err(error);
                        }
                    }
                }
            };

            let status = response.status();
            let headers = response.headers().clone();
            let meta = ResponseMeta::from_response_parts(
                request.operation().map(ToOwned::to_owned),
                url.clone(),
                status,
                &headers,
                &self.request_id_header_name,
                attempt + 1,
                started_at.elapsed(),
            );
            let body = response.text().await.map_err(|error| {
                let error_meta = ErrorMeta::from_response_meta(meta.clone(), String::new());
                let error = Error::from_reqwest(error, Some(error_meta.clone()));
                self.observer.on_error(&ErrorEvent {
                    meta: Some(error_meta),
                });
                error
            })?;

            match self.retry_config.classify_response(
                &request.method(),
                status,
                attempt,
                meta.retry_after(),
                started_at.elapsed(),
            ) {
                RetryDecision::RetryAfter(wait) => {
                    self.observer.on_retry(&RetryEvent {
                        operation: request.operation().map(ToOwned::to_owned),
                        method: request.method(),
                        url: url.clone(),
                        attempt: attempt + 1,
                        status: Some(status),
                        wait,
                    });
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                    continue;
                }
                RetryDecision::DoNotRetry => {}
            }

            if status.is_success() {
                return Ok(ResponseParts { meta, body });
            }

            let error_meta = ErrorMeta::from_response_meta(meta, body);
            let error = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Error::RateLimited(error_meta.clone())
            } else {
                Error::HttpStatus(error_meta.clone())
            };
            self.observer.on_error(&ErrorEvent {
                meta: Some(error_meta),
            });
            return Err(error);
        }
    }

    fn build_request(
        &self,
        url: &str,
        request: &RequestParts,
        authenticator: Option<&dyn Authenticator>,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let mut headers = self.default_headers.clone();
        headers.extend(request.headers().clone());
        if let Some(authenticator) = authenticator {
            authenticator.apply(&mut headers)?;
        }

        let mut builder = self
            .client
            .request(request.method(), url)
            .headers(headers)
            .query(request.query());

        builder = match request.body() {
            RequestBody::Empty => builder,
            RequestBody::Json(value) => builder.json(value),
            RequestBody::Text(value) => builder.body(value.clone()),
            RequestBody::Bytes(value) => builder.body(value.clone()),
        };

        if matches!(request.body(), RequestBody::Text(_))
            && !request.headers().contains_key(CONTENT_TYPE)
            && !self.default_headers.contains_key(CONTENT_TYPE)
        {
            builder = builder.header(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        }

        Ok(builder)
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self {
            reqwest_client: None,
            timeout: Duration::from_secs(30),
            default_headers: HeaderMap::new(),
            request_id_header_name: HeaderName::from_static("x-request-id"),
            retry_config: RetryConfig::default(),
            observer: Arc::new(NoopObserver),
            concurrency_limit: ConcurrencyLimit::default(),
        }
    }
}

impl HttpClientBuilder {
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn reqwest_client(mut self, client: reqwest::Client) -> Self {
        self.reqwest_client = Some(client);
        self
    }

    pub fn default_header(mut self, name: &str, value: &str) -> Result<Self, Error> {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            Error::InvalidRequest(format!("invalid default header name: {error}"))
        })?;
        let value = HeaderValue::from_str(value).map_err(|error| {
            Error::InvalidRequest(format!("invalid default header value: {error}"))
        })?;
        self.default_headers.insert(name, value);
        Ok(self)
    }

    pub fn request_id_header_name(mut self, name: &str) -> Result<Self, Error> {
        self.request_id_header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            Error::InvalidRequest(format!("invalid request id header name: {error}"))
        })?;
        Ok(self)
    }

    #[must_use]
    pub fn retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    #[must_use]
    pub fn observer(mut self, observer: Arc<dyn TransportObserver>) -> Self {
        self.observer = observer;
        self
    }

    #[must_use]
    pub fn concurrency_limit(mut self, concurrency_limit: ConcurrencyLimit) -> Self {
        self.concurrency_limit = concurrency_limit;
        self
    }

    pub fn build(self) -> Result<HttpClient, Error> {
        let client = match self.reqwest_client {
            Some(client) => client,
            None => reqwest::Client::builder()
                .timeout(self.timeout)
                .build()
                .map_err(|error| Error::from_reqwest(error, None))?,
        };

        Ok(HttpClient {
            client,
            default_headers: self.default_headers,
            request_id_header_name: self.request_id_header_name,
            retry_config: self.retry_config,
            observer: self.observer,
            concurrency_limit: self.concurrency_limit,
        })
    }
}
