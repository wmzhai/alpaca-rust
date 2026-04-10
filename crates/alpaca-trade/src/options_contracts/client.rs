use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    options_contracts::{ListRequest, ListResponse, OptionContract},
    pagination,
};

#[derive(Clone)]
pub struct OptionsContractsClient {
    inner: Arc<ClientInner>,
}

impl OptionsContractsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<ListResponse, Error> {
        let request = RequestParts::new(Method::GET, "/v2/options/contracts")
            .with_operation("options_contracts.list")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<ListResponse>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn list_all(&self, request: ListRequest) -> Result<ListResponse, Error> {
        pagination::collect_all(request, move |request| self.list(request)).await
    }

    pub async fn get(&self, symbol_or_id: &str) -> Result<OptionContract, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/options/contracts/{}",
                super::request::validate_symbol_or_id(symbol_or_id)?
            ),
        )
        .with_operation("options_contracts.get");

        self.inner
            .send_json::<OptionContract>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for OptionsContractsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OptionsContractsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
