use std::sync::Arc;

use crate::{corporate_actions::CorporateActionsClient, news::NewsClient, options::OptionsClient, stocks::StocksClient};

pub const DATA_API_KEY_ENV: &str = "ALPACA_DATA_API_KEY";
pub const DATA_SECRET_KEY_ENV: &str = "ALPACA_DATA_SECRET_KEY";
pub const DATA_BASE_URL_ENV: &str = "ALPACA_DATA_BASE_URL";
pub const LEGACY_DATA_BASE_URL_ENV: &str = "APCA_API_DATA_URL";
pub const DEFAULT_DATA_BASE_URL: &str = "https://data.alpaca.markets";

#[derive(Clone, Default)]
pub struct Client {
    inner: Arc<ClientInner>,
}

#[derive(Debug, Default)]
pub(crate) struct ClientInner;

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder;

impl Client {
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder
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
