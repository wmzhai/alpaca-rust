use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    portfolio_history::{GetRequest, PortfolioHistory},
};

#[derive(Clone)]
pub struct PortfolioHistoryClient {
    inner: Arc<ClientInner>,
}

impl PortfolioHistoryClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn get(&self, request: GetRequest) -> Result<PortfolioHistory, Error> {
        let request = RequestParts::new(Method::GET, "/v2/account/portfolio/history")
            .with_operation("portfolio_history.get")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<PortfolioHistory>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for PortfolioHistoryClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortfolioHistoryClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
