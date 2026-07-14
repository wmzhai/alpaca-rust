use std::{collections::HashSet, sync::Mutex};

use alpaca_http::{RequestStart, ResponseEvent, ResponseMeta, RetryEvent, TransportObserver};

#[derive(Debug, Default)]
pub struct LiveRequestObserver {
    requests: Mutex<Vec<RequestStart>>,
    retries: Mutex<Vec<RetryEvent>>,
    responses: Mutex<Vec<ResponseMeta>>,
}

impl LiveRequestObserver {
    #[must_use]
    pub fn requests(&self) -> Vec<RequestStart> {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn responses(&self) -> Vec<ResponseMeta> {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn retries(&self) -> Vec<RetryEvent> {
        self.retries
            .lock()
            .expect("live retry observer mutex should not be poisoned")
            .clone()
    }

    #[must_use]
    pub fn last_request(&self) -> Option<RequestStart> {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .last()
            .cloned()
    }

    #[must_use]
    pub fn last_response(&self) -> Option<ResponseMeta> {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .last()
            .cloned()
    }
}

impl TransportObserver for LiveRequestObserver {
    fn on_request_start(&self, event: &RequestStart) {
        self.requests
            .lock()
            .expect("live request observer mutex should not be poisoned")
            .push(event.clone());
    }

    fn on_retry(&self, event: &RetryEvent) {
        self.retries
            .lock()
            .expect("live retry observer mutex should not be poisoned")
            .push(event.clone());
    }

    fn on_response(&self, event: &ResponseEvent) {
        self.responses
            .lock()
            .expect("live response observer mutex should not be poisoned")
            .push(event.meta.clone());
    }
}

#[must_use]
pub fn observed_query(request: &RequestStart) -> Vec<(String, String)> {
    reqwest::Url::parse(&request.url)
        .expect("observed request URL should be valid")
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[must_use]
pub fn observed_request_lines(requests: &[RequestStart]) -> Vec<String> {
    requests
        .iter()
        .map(|request| format!("{} {}", request.method, request.url))
        .collect()
}

#[must_use]
pub fn unique_observed_requests(requests: &[RequestStart]) -> Vec<RequestStart> {
    let mut seen = HashSet::new();
    requests
        .iter()
        .filter(|request| seen.insert((request.method.clone(), request.url.clone())))
        .cloned()
        .collect()
}

#[must_use]
pub fn observed_query_value(request: &RequestStart, key: &str) -> Option<String> {
    reqwest::Url::parse(&request.url)
        .expect("observed request URL should be valid")
        .query_pairs()
        .find_map(|(candidate, value)| (candidate == key).then(|| value.into_owned()))
}
