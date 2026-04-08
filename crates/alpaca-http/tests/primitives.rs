use std::sync::Arc;
use std::time::Duration;

use alpaca_http::{
    ConcurrencyLimit, ErrorMeta, NoContent, NoopObserver, RequestBody, RequestParts,
    RequestStart, ResponseMeta, RetryConfig, RetryDecision, RetryEvent,
    StaticHeaderAuthenticator, TransportObserver,
};
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

#[test]
fn request_parts_keep_path_query_headers_and_body() {
    let mut headers = HeaderMap::new();
    headers.insert("x-test-header", HeaderValue::from_static("demo"));

    let parts = RequestParts::new(Method::POST, "/v2/orders")
        .with_operation("orders.create")
        .with_query([(String::from("symbol"), String::from("AAPL"))])
        .with_headers(headers.clone())
        .with_json_body(serde_json::json!({"symbol": "AAPL"}));

    assert_eq!(parts.method(), Method::POST);
    assert_eq!(parts.path(), "/v2/orders");
    assert_eq!(parts.operation(), Some("orders.create"));
    assert_eq!(parts.query(), &[(String::from("symbol"), String::from("AAPL"))]);
    assert_eq!(parts.headers().get("x-test-header"), headers.get("x-test-header"));
    assert!(matches!(parts.body(), RequestBody::Json(_)));
    assert_eq!(NoContent, NoContent::default());
}

#[test]
fn static_header_authenticator_injects_headers() {
    let authenticator = StaticHeaderAuthenticator::from_pairs([
        ("apca-api-key-id", "key-id"),
        ("apca-api-secret-key", "secret-key"),
    ])
    .expect("authenticator should build");
    let mut headers = HeaderMap::new();

    authenticator.apply(&mut headers).expect("header injection should succeed");

    assert_eq!(headers.get("apca-api-key-id"), Some(&HeaderValue::from_static("key-id")));
    assert_eq!(
        headers.get("apca-api-secret-key"),
        Some(&HeaderValue::from_static("secret-key"))
    );
}

#[test]
fn retry_config_only_retries_enabled_methods_and_statuses() {
    let config = RetryConfig::default().with_retryable_methods([Method::GET, Method::DELETE]);

    assert_eq!(
        config.classify_response(
            &Method::GET,
            StatusCode::TOO_MANY_REQUESTS,
            0,
            Some(Duration::from_secs(2)),
            Duration::ZERO,
        ),
        RetryDecision::DoNotRetry,
    );

    let config = config
        .with_retry_on_429(true)
        .with_respect_retry_after(true);

    assert_eq!(
        config.classify_response(
            &Method::GET,
            StatusCode::TOO_MANY_REQUESTS,
            0,
            Some(Duration::from_secs(2)),
            Duration::ZERO,
        ),
        RetryDecision::RetryAfter(Duration::from_secs(2)),
    );
    assert_eq!(
        config.classify_response(
            &Method::POST,
            StatusCode::INTERNAL_SERVER_ERROR,
            0,
            None,
            Duration::ZERO,
        ),
        RetryDecision::DoNotRetry,
    );
}

#[test]
fn response_meta_and_error_meta_extract_request_id_retry_after_and_body_snippet() {
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", HeaderValue::from_static("req-123"));
    headers.insert("retry-after", HeaderValue::from_static("3"));

    let meta = ResponseMeta::from_response_parts(
        Some("orders.list".to_owned()),
        "https://paper-api.alpaca.markets/v2/orders".to_owned(),
        StatusCode::TOO_MANY_REQUESTS,
        &headers,
        &HeaderName::from_static("x-request-id"),
        1,
        Duration::from_millis(250),
    );
    let error_meta = ErrorMeta::from_response_meta(meta.clone(), "x".repeat(300));

    assert_eq!(meta.operation(), Some("orders.list"));
    assert_eq!(meta.request_id(), Some("req-123"));
    assert_eq!(meta.retry_after(), Some(Duration::from_secs(3)));
    assert_eq!(error_meta.request_id(), Some("req-123"));
    assert_eq!(error_meta.retry_after(), Some(Duration::from_secs(3)));
    assert_eq!(error_meta.body_snippet().map(str::len), Some(259));
}

#[tokio::test]
async fn concurrency_limit_blocks_until_previous_permit_is_dropped() {
    let limit = ConcurrencyLimit::new(Some(1));
    let permit = limit.acquire().await.expect("first permit should succeed");

    let blocked = tokio::time::timeout(Duration::from_millis(20), limit.acquire()).await;
    assert!(blocked.is_err(), "second acquire should block while first permit is held");

    drop(permit);

    let _second = limit.acquire().await.expect("second permit should succeed after drop");
}

#[test]
fn noop_observer_accepts_all_events() {
    let observer: Arc<dyn TransportObserver> = Arc::new(NoopObserver);

    observer.on_request_start(&RequestStart {
        operation: Some("orders.list".to_owned()),
        method: Method::GET,
        url: "https://paper-api.alpaca.markets/v2/orders".to_owned(),
    });
    observer.on_retry(&RetryEvent {
        operation: Some("orders.list".to_owned()),
        method: Method::GET,
        url: "https://paper-api.alpaca.markets/v2/orders".to_owned(),
        attempt: 1,
        status: Some(StatusCode::TOO_MANY_REQUESTS),
        wait: Duration::from_millis(50),
    });
}
