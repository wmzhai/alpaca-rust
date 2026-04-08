use reqwest::{Method, header::HeaderMap};

#[derive(Debug, Clone, PartialEq)]
pub enum RequestBody {
    Empty,
    Json(serde_json::Value),
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct RequestParts {
    operation: Option<String>,
    method: Method,
    path: String,
    query: Vec<(String, String)>,
    headers: HeaderMap,
    body: RequestBody,
}

impl RequestParts {
    #[must_use]
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            operation: None,
            method,
            path: path.into(),
            query: Vec::new(),
            headers: HeaderMap::new(),
            body: RequestBody::Empty,
        }
    }

    #[must_use]
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    #[must_use]
    pub fn with_query<I>(mut self, query: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.query = query.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    #[must_use]
    pub fn with_json_body(mut self, body: serde_json::Value) -> Self {
        self.body = RequestBody::Json(body);
        self
    }

    #[must_use]
    pub fn with_text_body(mut self, body: impl Into<String>) -> Self {
        self.body = RequestBody::Text(body.into());
        self
    }

    #[must_use]
    pub fn with_bytes_body(mut self, body: Vec<u8>) -> Self {
        self.body = RequestBody::Bytes(body);
        self
    }

    #[must_use]
    pub fn operation(&self) -> Option<&str> {
        self.operation.as_deref()
    }

    #[must_use]
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn query(&self) -> &[(String, String)] {
        &self.query
    }

    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[must_use]
    pub fn body(&self) -> &RequestBody {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoContent;
