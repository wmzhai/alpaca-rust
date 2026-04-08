use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{Error, clock::Clock};

#[derive(Clone)]
pub struct ClockClient {
    inner: Arc<ClientInner>,
}

impl ClockClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn get(&self) -> Result<Clock, Error> {
        let request = RequestParts::new(Method::GET, "/v2/clock").with_operation("clock.get");

        self.inner
            .send_json::<Clock>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for ClockClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClockClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
