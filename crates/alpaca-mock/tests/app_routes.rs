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

#[tokio::test]
async fn reset_route_alias_clears_mock_state() {
    let state = MockServerState::new();
    let app = alpaca_mock::build_app_with_state(state);

    let account_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key-reset")
                .header("apca-api-secret-key", "mock-secret-reset")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("account request should succeed");
    assert_eq!(account_response.status(), StatusCode::OK);

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
                        "message": "reset alias outage"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("set fault request should succeed");
    assert_eq!(set_fault_response.status(), StatusCode::OK);

    let reset_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reset")
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
}
