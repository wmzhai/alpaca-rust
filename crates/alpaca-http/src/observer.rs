use std::time::Duration;

use reqwest::{Method, StatusCode};

use crate::meta::{ErrorMeta, ResponseMeta};

pub trait TransportObserver: Send + Sync {
    fn on_request_start(&self, _event: &RequestStart) {}
    fn on_retry(&self, _event: &RetryEvent) {}
    fn on_response(&self, _event: &ResponseEvent) {}
    fn on_error(&self, _event: &ErrorEvent) {}
}

#[derive(Debug, Default)]
pub struct NoopObserver;

impl TransportObserver for NoopObserver {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestStart {
    pub operation: Option<String>,
    pub method: Method,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryEvent {
    pub operation: Option<String>,
    pub method: Method,
    pub url: String,
    pub attempt: u32,
    pub status: Option<StatusCode>,
    pub wait: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseEvent {
    pub meta: ResponseMeta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorEvent {
    pub meta: Option<ErrorMeta>,
}
