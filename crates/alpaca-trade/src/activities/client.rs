use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    activities::{Activity, ListByTypeRequest, ListRequest},
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

    pub async fn list_all(&self, request: ListRequest) -> Result<Vec<Activity>, Error> {
        collect_all_activity_pages(request, move |request| self.list(request)).await
    }

    pub async fn list_by_type(
        &self,
        activity_type: &str,
        request: ListByTypeRequest,
    ) -> Result<Vec<Activity>, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/account/activities/{}",
                super::request::validate_activity_type(activity_type)?
            ),
        )
        .with_operation("activities.list_by_type")
        .with_query(request.into_query()?);

        self.inner
            .send_json::<Vec<Activity>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn list_by_type_all(
        &self,
        activity_type: &str,
        request: ListByTypeRequest,
    ) -> Result<Vec<Activity>, Error> {
        let activity_type = super::request::validate_activity_type(activity_type)?;
        collect_all_activity_pages(request, move |request| {
            let activity_type = activity_type.clone();
            async move { self.list_by_type(&activity_type, request).await }
        })
        .await
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

trait ActivityPageRequest: Clone {
    fn with_page_token(&self, page_token: Option<String>) -> Self;
    fn page_size(&self) -> Option<u32>;
    fn date(&self) -> Option<&str>;
}

impl ActivityPageRequest for ListRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }

    fn page_size(&self) -> Option<u32> {
        self.page_size
    }

    fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }
}

impl ActivityPageRequest for ListByTypeRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }

    fn page_size(&self) -> Option<u32> {
        self.page_size
    }

    fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }
}

async fn collect_all_activity_pages<Request, Fetch, FutureOutput>(
    initial_request: Request,
    mut fetch_page: Fetch,
) -> Result<Vec<Activity>, Error>
where
    Request: ActivityPageRequest,
    Fetch: FnMut(Request) -> FutureOutput,
    FutureOutput: Future<Output = Result<Vec<Activity>, Error>>,
{
    let page_size = effective_activity_page_size(&initial_request);
    let mut combined = fetch_page(initial_request.clone()).await?;
    let Some(page_size) = page_size else {
        return Ok(combined);
    };

    let mut seen_page_tokens = HashSet::new();
    let mut current_page_len = combined.len();

    while current_page_len >= page_size {
        let Some(page_token) = combined
            .last()
            .map(|activity| activity.id.clone())
            .filter(|id| !id.is_empty())
        else {
            break;
        };

        if !seen_page_tokens.insert(page_token.clone()) {
            return Err(Error::InvalidRequest(format!(
                "pagination contract violation: repeated page_token `{page_token}`"
            )));
        }

        let next_page = fetch_page(initial_request.with_page_token(Some(page_token))).await?;
        if let Some(next_page_token) = next_page
            .last()
            .map(|activity| activity.id.as_str())
            .filter(|id| !id.is_empty())
            && seen_page_tokens.contains(next_page_token)
        {
            return Err(Error::InvalidRequest(format!(
                "pagination contract violation: repeated page_token `{next_page_token}`"
            )));
        }

        if next_page.is_empty() {
            break;
        }

        current_page_len = next_page.len();
        combined.extend(next_page);
        if current_page_len < page_size {
            break;
        }
    }

    Ok(combined)
}

fn effective_activity_page_size<Request: ActivityPageRequest>(request: &Request) -> Option<usize> {
    request
        .page_size()
        .map(|page_size| page_size as usize)
        .or_else(|| {
            // Alpaca documents a default/maximum page size of 100 when `date` is absent.
            request.date().is_none().then_some(100)
        })
}
