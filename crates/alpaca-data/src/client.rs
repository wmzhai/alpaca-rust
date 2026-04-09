use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use alpaca_core::{BaseUrl, Credentials, env};
use alpaca_http::{
    ConcurrencyLimit, HttpClient, HttpResponse, NoContent, RequestParts, RetryConfig,
    StaticHeaderAuthenticator, TransportObserver,
};
use serde::de::DeserializeOwned;

use crate::{
    Error, corporate_actions::CorporateActionsClient, news::NewsClient, options::OptionsClient,
    stocks::StocksClient,
};

pub const DATA_API_KEY_ENV: &str = "ALPACA_DATA_API_KEY";
pub const DATA_SECRET_KEY_ENV: &str = "ALPACA_DATA_SECRET_KEY";
pub const DATA_BASE_URL_ENV: &str = "ALPACA_DATA_BASE_URL";
pub const LEGACY_DATA_BASE_URL_ENV: &str = "APCA_API_DATA_URL";
pub const DEFAULT_DATA_BASE_URL: &str = "https://data.alpaca.markets";
const APCA_API_KEY_HEADER: &str = "APCA-API-KEY-ID";
const APCA_API_SECRET_HEADER: &str = "APCA-API-SECRET-KEY";

#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

#[allow(dead_code)]
pub(crate) struct ClientInner {
    http: HttpClient,
    auth: StaticHeaderAuthenticator,
    base_url: BaseUrl,
}

#[derive(Clone, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    secret_key: Option<String>,
    base_url: Option<BaseUrl>,
    timeout: Option<Duration>,
    reqwest_client: Option<reqwest::Client>,
    observer: Option<Arc<dyn TransportObserver>>,
    retry_config: RetryConfig,
    max_in_flight: Option<usize>,
}

impl Client {
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn new(credentials: Credentials) -> Result<Self, Error> {
        Self::builder().credentials(credentials).build()
    }

    pub fn from_env() -> Result<Self, Error> {
        Self::builder()
            .credentials_from_env()?
            .base_url_from_env()?
            .build()
    }

    #[must_use]
    pub fn base_url(&self) -> &BaseUrl {
        self.inner.base_url()
    }

    #[must_use]
    pub fn stocks(&self) -> StocksClient {
        StocksClient::new(self.inner.clone())
    }

    #[must_use]
    pub fn options(&self) -> OptionsClient {
        OptionsClient::new(self.inner.clone())
    }

    #[must_use]
    pub fn news(&self) -> NewsClient {
        NewsClient::new(self.inner.clone())
    }

    #[must_use]
    pub fn corporate_actions(&self) -> CorporateActionsClient {
        CorporateActionsClient::new(self.inner.clone())
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", self.inner.base_url())
            .field("http", &"HttpClient")
            .field("auth", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for ClientInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientInner")
            .field("base_url", &self.base_url)
            .field("http", &"HttpClient")
            .field("auth", &"[REDACTED]")
            .finish()
    }
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &redacted_option(&self.api_key))
            .field("secret_key", &redacted_option(&self.secret_key))
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field(
                "reqwest_client",
                &self.reqwest_client.as_ref().map(|_| "reqwest::Client"),
            )
            .field(
                "observer",
                &self.observer.as_ref().map(|_| "TransportObserver"),
            )
            .field("retry_config", &self.retry_config)
            .field("max_in_flight", &self.max_in_flight)
            .finish()
    }
}

impl ClientBuilder {
    #[must_use]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.api_key = Some(credentials.api_key().to_owned());
        self.secret_key = Some(credentials.secret_key().to_owned());
        self
    }

    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    #[must_use]
    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    #[must_use]
    pub fn base_url(mut self, base_url: BaseUrl) -> Self {
        self.base_url = Some(base_url);
        self
    }

    pub fn base_url_str(mut self, base_url: impl AsRef<str>) -> Result<Self, Error> {
        self.base_url = Some(BaseUrl::new(base_url.as_ref())?);
        Ok(self)
    }

    pub fn credentials_from_env(self) -> Result<Self, Error> {
        self.credentials_from_env_names(DATA_API_KEY_ENV, DATA_SECRET_KEY_ENV)
    }

    pub fn credentials_from_env_names(
        mut self,
        api_key_var: &str,
        secret_key_var: &str,
    ) -> Result<Self, Error> {
        if let Some(credentials) = env::credentials_from_env_names(api_key_var, secret_key_var)? {
            return Ok(self.credentials(credentials));
        }

        if let Some(credentials) = env::credentials_from_env()? {
            self = self.credentials(credentials);
        }

        Ok(self)
    }

    pub fn base_url_from_env(mut self) -> Result<Self, Error> {
        if let Some(base_url) = env::base_url_from_env_name(DATA_BASE_URL_ENV)? {
            self.base_url = Some(base_url);
            return Ok(self);
        }

        if let Some(base_url) = env::base_url_from_env_name(LEGACY_DATA_BASE_URL_ENV)? {
            self.base_url = Some(base_url);
        }

        Ok(self)
    }

    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn reqwest_client(mut self, reqwest_client: reqwest::Client) -> Self {
        self.reqwest_client = Some(reqwest_client);
        self
    }

    #[must_use]
    pub fn observer(mut self, observer: Arc<dyn TransportObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    #[must_use]
    pub fn retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    #[must_use]
    pub fn max_in_flight(mut self, max_in_flight: usize) -> Self {
        self.max_in_flight = Some(max_in_flight);
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        if self.reqwest_client.is_some() && self.timeout.is_some() {
            return Err(Error::InvalidConfiguration(
                "reqwest_client owns timeout configuration; remove timeout(...) or configure timeout on the injected reqwest::Client".to_owned(),
            ));
        }

        let credentials = match (self.api_key, self.secret_key) {
            (Some(api_key), Some(secret_key)) => Credentials::new(api_key, secret_key)?,
            (None, None) => return Err(Error::MissingCredentials),
            _ => {
                return Err(Error::InvalidConfiguration(
                    "api_key and secret_key must be paired".to_owned(),
                ));
            }
        };

        let base_url = match self.base_url {
            Some(base_url) => base_url,
            None => BaseUrl::new(DEFAULT_DATA_BASE_URL)?,
        };
        let auth = StaticHeaderAuthenticator::from_pairs([
            (APCA_API_KEY_HEADER, credentials.api_key()),
            (APCA_API_SECRET_HEADER, credentials.secret_key()),
        ])?;

        let mut http_builder = HttpClient::builder().retry_config(self.retry_config);
        if let Some(timeout) = self.timeout {
            http_builder = http_builder.timeout(timeout);
        }
        if let Some(reqwest_client) = self.reqwest_client {
            http_builder = http_builder.reqwest_client(reqwest_client);
        }
        if let Some(observer) = self.observer {
            http_builder = http_builder.observer(observer);
        }
        if let Some(max_in_flight) = self.max_in_flight {
            http_builder =
                http_builder.concurrency_limit(ConcurrencyLimit::new(Some(max_in_flight)));
        }

        let http = http_builder.build()?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                auth,
                base_url,
            }),
        })
    }
}

impl ClientInner {
    #[allow(dead_code)]
    pub(crate) async fn send_json<T>(&self, request: RequestParts) -> Result<HttpResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        self.http
            .send_json(&self.base_url, request, Some(&self.auth))
            .await
            .map_err(Error::from)
    }

    #[allow(dead_code)]
    pub(crate) async fn send_text(
        &self,
        request: RequestParts,
    ) -> Result<HttpResponse<String>, Error> {
        self.http
            .send_text(&self.base_url, request, Some(&self.auth))
            .await
            .map_err(Error::from)
    }

    #[allow(dead_code)]
    pub(crate) async fn send_no_content(
        &self,
        request: RequestParts,
    ) -> Result<HttpResponse<NoContent>, Error> {
        self.http
            .send_no_content(&self.base_url, request, Some(&self.auth))
            .await
            .map_err(Error::from)
    }

    #[must_use]
    pub(crate) fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }
}

fn redacted_option(value: &Option<String>) -> &'static str {
    match value {
        Some(_) => "[REDACTED]",
        None => "None",
    }
}
