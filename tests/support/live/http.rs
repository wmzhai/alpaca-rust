use alpaca_core::{BaseUrl, Credentials};
use std::{collections::HashSet, sync::Mutex};

use alpaca_http::{
    HttpClient, RequestParts, RequestStart, ResponseEvent, ResponseMeta, RetryEvent,
    StaticHeaderAuthenticator, TransportObserver,
};
use reqwest::Method;
use serde_json::Value;

use super::{SupportError, TradeServiceConfig};

#[derive(Debug, Default)]
pub struct LiveRequestObserver {
    requests: Mutex<Vec<RequestStart>>,
    retries: Mutex<Vec<RetryEvent>>,
    responses: Mutex<Vec<ResponseMeta>>,
}

impl LiveRequestObserver {
    #[must_use]
    pub fn requests(&self) -> Vec<RequestStart> {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn responses(&self) -> Vec<ResponseMeta> {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn retries(&self) -> Vec<RetryEvent> {
        self.retries
            .lock()
            .expect("live retry observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn last_request(&self) -> Option<RequestStart> {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .last()
            .cloned()
    }

    #[must_use]
    pub fn last_response(&self) -> Option<ResponseMeta> {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .last()
            .cloned()
    }
}

impl TransportObserver for LiveRequestObserver {
    fn on_request_start(&self, event: &RequestStart) {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .push(event.clone());
    }

    fn on_retry(&self, event: &RetryEvent) {
        self.retries
            .lock()
            .expect("live retry observer mutex should not be poisoned")
            .push(event.clone());
    }

    fn on_response(&self, event: &ResponseEvent) {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .push(event.meta.clone());
    }
}

#[must_use]
pub fn observed_query(request: &RequestStart) -> Vec<(String, String)> {
    reqwest::Url::parse(&request.url)
        .expect("observed request URL should be valid")
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[must_use]
pub fn observed_request_lines(requests: &[RequestStart]) -> Vec<String> {
    requests
        .iter()
        .map(|request| format!("{} {}", request.method, request.url))
        .collect()
}

#[must_use]
pub fn unique_observed_requests(requests: &[RequestStart]) -> Vec<RequestStart> {
    let mut seen = HashSet::new();
    requests
        .iter()
        .filter(|request| seen.insert((request.method.clone(), request.url.clone())))
        .cloned()
        .collect()
}

#[must_use]
pub fn observed_query_value(request: &RequestStart, key: &str) -> Option<String> {
    reqwest::Url::parse(&request.url)
        .expect("observed request URL should be valid")
        .query_pairs()
        .find_map(|(candidate, value)| (candidate == key).then(|| value.into_owned()))
}

#[derive(Debug, Clone)]
pub struct JsonProbeResponse {
    body: Value,
    meta: ResponseMeta,
}

impl JsonProbeResponse {
    #[must_use]
    pub fn body(&self) -> &Value {
        &self.body
    }

    #[must_use]
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }

    #[must_use]
    pub fn into_parts(self) -> (Value, ResponseMeta) {
        (self.body, self.meta)
    }
}

#[derive(Clone)]
pub struct LiveHttpProbe {
    client: HttpClient,
}

impl LiveHttpProbe {
    pub fn new() -> Result<Self, SupportError> {
        Ok(Self::from_client(HttpClient::builder().build()?))
    }

    #[must_use]
    pub fn from_client(client: HttpClient) -> Self {
        Self { client }
    }

    pub async fn get_trade_json<I, K, V>(
        &self,
        service: &TradeServiceConfig,
        path: &str,
        query: I,
    ) -> Result<JsonProbeResponse, SupportError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        self.get_json_with_base_url(
            service.credentials(),
            service.base_url().clone(),
            path,
            query,
        )
        .await
    }

    async fn get_json_with_base_url<I, K, V>(
        &self,
        credentials: &Credentials,
        base_url: BaseUrl,
        path: &str,
        query: I,
    ) -> Result<JsonProbeResponse, SupportError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        let request = RequestParts::new(Method::GET, path).with_query(
            query
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<Vec<_>>(),
        );
        let auth = StaticHeaderAuthenticator::from_pairs([
            ("APCA-API-KEY-ID", credentials.api_key()),
            ("APCA-API-SECRET-KEY", credentials.secret_key()),
        ])?;
        let response = self
            .client
            .send_json::<Value>(&base_url, request, Some(&auth))
            .await?;
        let (body, meta) = response.into_parts();

        Ok(JsonProbeResponse { body, meta })
    }
}
