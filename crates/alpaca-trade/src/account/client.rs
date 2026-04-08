use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{Error, account::Account};

#[derive(Clone)]
pub struct AccountClient {
    inner: Arc<ClientInner>,
}

impl AccountClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn get(&self) -> Result<Account, Error> {
        let request = RequestParts::new(Method::GET, "/v2/account").with_operation("account.get");

        self.inner
            .send_json::<Account>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for AccountClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
