use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::util::ServiceExt;

use alpaca_mock::MockServerState;

#[tokio::test]
async fn health_returns_ok() {
    let app = alpaca_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn account_route_requires_apca_api_headers() {
    let app = alpaca_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn account_route_projects_per_key_account_state() {
    let app = alpaca_mock::build_app();

    let response_a = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key-a")
                .header("apca-api-secret-key", "mock-secret-a")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("account A request should succeed");
    assert_eq!(response_a.status(), StatusCode::OK);
    let body_a = to_bytes(response_a.into_body(), usize::MAX)
        .await
        .expect("account A body should read");
    let account_a: alpaca_trade::account::Account =
        serde_json::from_slice(&body_a).expect("account A should deserialize");

    let response_b = app
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key-b")
                .header("apca-api-secret-key", "mock-secret-b")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("account B request should succeed");
    assert_eq!(response_b.status(), StatusCode::OK);
    let body_b = to_bytes(response_b.into_body(), usize::MAX)
        .await
        .expect("account B body should read");
    let account_b: alpaca_trade::account::Account =
        serde_json::from_slice(&body_b).expect("account B should deserialize");

    assert_ne!(account_a.id, account_b.id);
    assert_ne!(account_a.account_number, account_b.account_number);
    assert_eq!(account_a.status, "ACTIVE");
    assert_eq!(account_b.status, "ACTIVE");
}

#[tokio::test]
async fn admin_fault_and_reset_routes_control_mock_state() {
    let state = MockServerState::new();
    let app = alpaca_mock::build_app_with_state(state);

    let set_fault_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/faults/http")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "status": 503,
                        "message": "phase-11 injected outage"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("set fault request should succeed");
    assert_eq!(set_fault_response.status(), StatusCode::OK);

    let faulted_account_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key-a")
                .header("apca-api-secret-key", "mock-secret-a")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("faulted account request should succeed");
    assert_eq!(
        faulted_account_response.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );

    let reset_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/reset")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("reset request should succeed");
    assert_eq!(reset_response.status(), StatusCode::OK);
    let reset_body = to_bytes(reset_response.into_body(), usize::MAX)
        .await
        .expect("reset body should read");
    let reset_state: Value =
        serde_json::from_slice(&reset_body).expect("reset state should deserialize");
    assert_eq!(reset_state["account_count"], json!(0));
    assert!(reset_state["http_fault"].is_null());

    let recovered_account_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key-a")
                .header("apca-api-secret-key", "mock-secret-a")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("recovered account request should succeed");
    assert_eq!(recovered_account_response.status(), StatusCode::OK);

    let admin_state_response = app
        .oneshot(
            Request::builder()
                .uri("/admin/state")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("admin state request should succeed");
    assert_eq!(admin_state_response.status(), StatusCode::OK);
}
