use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    calendar::{Calendar, ListRequest},
};

#[derive(Clone)]
pub struct CalendarClient {
    inner: Arc<ClientInner>,
}

impl CalendarClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Calendar>, Error> {
        let request = RequestParts::new(Method::GET, "/v2/calendar")
            .with_operation("calendar.list")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<Vec<Calendar>>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for CalendarClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
