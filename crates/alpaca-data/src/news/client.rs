use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::{Error, client::ClientInner, pagination};

use super::{ListRequest, ListResponse};

#[derive(Clone)]
pub struct NewsClient {
    inner: Arc<ClientInner>,
}

impl NewsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<ListResponse, Error> {
        request.validate()?;
        self.get_json("news.list", "/v1beta1/news", request.into_query())
            .await
    }

    pub async fn list_all(&self, request: ListRequest) -> Result<ListResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.list(request).await }
        })
        .await
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }

    async fn get_json<Response>(
        &self,
        operation: &'static str,
        path: impl Into<String>,
        query: Vec<(String, String)>,
    ) -> Result<Response, Error>
    where
        Response: DeserializeOwned,
    {
        let request = RequestParts::new(Method::GET, path.into())
            .with_operation(operation)
            .with_query(query);

        self.inner
            .send_json::<Response>(request)
            .await
            .map(|response| response.into_body())
    }
}

impl fmt::Debug for NewsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewsClient").finish()
    }
}
