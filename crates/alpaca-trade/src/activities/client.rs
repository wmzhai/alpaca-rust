use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    activities::{Activity, ListRequest},
};

#[derive(Clone)]
pub struct ActivitiesClient {
    inner: Arc<ClientInner>,
}

impl ActivitiesClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Activity>, Error> {
        let request = RequestParts::new(Method::GET, "/v2/account/activities")
            .with_operation("activities.list")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<Vec<Activity>>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for ActivitiesClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivitiesClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
