use std::{collections::HashSet, future::Future};

use crate::Error;

pub trait PaginatedRequest: Clone {
    fn with_page_token(&self, page_token: Option<String>) -> Self;
}

pub trait PaginatedResponse: Sized {
    fn next_page_token(&self) -> Option<&str>;
    fn merge_page(&mut self, next: Self) -> Result<(), Error>;
    fn clear_next_page_token(&mut self);
}

pub async fn collect_all<Request, Response, Fetch, FutureOutput>(
    initial_request: Request,
    mut fetch_page: Fetch,
) -> Result<Response, Error>
where
    Request: PaginatedRequest,
    Response: PaginatedResponse,
    Fetch: FnMut(Request) -> FutureOutput,
    FutureOutput: Future<Output = Result<Response, Error>>,
{
    let mut combined = fetch_page(initial_request.clone()).await?;
    let mut seen_page_tokens = HashSet::new();

    while let Some(page_token) = combined.next_page_token().map(str::to_owned) {
        if !seen_page_tokens.insert(page_token.clone()) {
            return Err(Error::InvalidRequest(format!(
                "pagination contract violation: repeated next_page_token `{page_token}`"
            )));
        }

        let next_page = fetch_page(initial_request.with_page_token(Some(page_token))).await?;

        if let Some(next_page_token) = next_page.next_page_token()
            && seen_page_tokens.contains(next_page_token)
        {
            return Err(Error::InvalidRequest(format!(
                "pagination contract violation: repeated next_page_token `{next_page_token}`"
            )));
        }

        combined.merge_page(next_page)?;
    }

    combined.clear_next_page_token();
    Ok(combined)
}
