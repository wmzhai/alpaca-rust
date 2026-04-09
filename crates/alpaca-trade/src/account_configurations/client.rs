use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    account_configurations::{AccountConfigurations, UpdateRequest},
};

#[derive(Clone)]
pub struct AccountConfigurationsClient {
    inner: Arc<ClientInner>,
}

impl AccountConfigurationsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn get(&self) -> Result<AccountConfigurations, Error> {
        let request = RequestParts::new(Method::GET, "/v2/account/configurations")
            .with_operation("account_configurations.get");

        self.inner
            .send_json::<AccountConfigurations>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn update(&self, request: UpdateRequest) -> Result<AccountConfigurations, Error> {
        let request = RequestParts::new(Method::PATCH, "/v2/account/configurations")
            .with_operation("account_configurations.update")
            .with_json_body(request.into_json()?);

        self.inner
            .send_json::<AccountConfigurations>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for AccountConfigurationsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountConfigurationsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
