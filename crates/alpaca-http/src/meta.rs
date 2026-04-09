use std::time::Duration;

use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderName, RETRY_AFTER},
};

const MAX_BODY_SNIPPET_CHARS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseMeta {
    operation: Option<String>,
    url: String,
    status: u16,
    request_id: Option<String>,
    attempt_count: u32,
    elapsed: Duration,
    retry_after: Option<Duration>,
}

impl ResponseMeta {
    #[must_use]
    pub fn from_response_parts(
        operation: Option<String>,
        url: String,
        status: StatusCode,
        headers: &HeaderMap,
        request_id_header: &HeaderName,
        attempt_count: u32,
        elapsed: Duration,
    ) -> Self {
        Self {
            operation,
            url,
            status: status.as_u16(),
            request_id: parse_header_string(headers, request_id_header),
            attempt_count,
            elapsed,
            retry_after: parse_retry_after(headers),
        }
    }

    #[must_use]
    pub fn operation(&self) -> Option<&str> {
        self.operation.as_deref()
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    #[must_use]
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorMeta {
    operation: Option<String>,
    url: String,
    status: u16,
    request_id: Option<String>,
    attempt_count: u32,
    elapsed: Duration,
    retry_after: Option<Duration>,
    body_snippet: Option<String>,
}

impl ErrorMeta {
    #[must_use]
    pub fn from_response_meta(meta: ResponseMeta, body: impl Into<String>) -> Self {
        Self {
            operation: meta.operation,
            url: meta.url,
            status: meta.status,
            request_id: meta.request_id,
            attempt_count: meta.attempt_count,
            elapsed: meta.elapsed,
            retry_after: meta.retry_after,
            body_snippet: snippet_body(body.into()),
        }
    }

    #[must_use]
    pub fn operation(&self) -> Option<&str> {
        self.operation.as_deref()
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    #[must_use]
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    #[must_use]
    pub fn body_snippet(&self) -> Option<&str> {
        self.body_snippet.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse<T> {
    body: T,
    meta: ResponseMeta,
}

impl<T> HttpResponse<T> {
    #[must_use]
    pub fn new(body: T, meta: ResponseMeta) -> Self {
        Self { body, meta }
    }

    #[must_use]
    pub fn body(&self) -> &T {
        &self.body
    }

    #[must_use]
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }

    #[must_use]
    pub fn into_body(self) -> T {
        self.body
    }

    #[must_use]
    pub fn into_parts(self) -> (T, ResponseMeta) {
        (self.body, self.meta)
    }
}

fn parse_header_string(headers: &HeaderMap, name: &HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn snippet_body(body: String) -> Option<String> {
    if body.is_empty() {
        return None;
    }

    let mut snippet: String = body.chars().take(MAX_BODY_SNIPPET_CHARS).collect();
    if snippet.len() < body.len() {
        snippet.push_str("...");
    }

    Some(snippet)
}
