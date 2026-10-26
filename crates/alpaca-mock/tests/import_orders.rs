use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::util::ServiceExt;

#[tokio::test]
async fn admin_import_route_keeps_order_id_and_new_status() {
    let app = alpaca_mock::build_app();
    let import = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/orders/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "api_key": "mock-key",
                        "orders": [{
                            "id": "57883507-70c4-4a1b-87e8-325e32c88700",
                            "client_order_id": "grid-submit-1",
                            "symbol": "AAPL",
                            "side": "buy",
                            "qty": "10",
                            "limit_price": "100.00"
                        }]
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("import request should succeed");
    assert_eq!(import.status(), StatusCode::OK);
    let import_body = to_bytes(import.into_body(), usize::MAX)
        .await
        .expect("import body should read");
    let import_json: Value = serde_json::from_slice(&import_body).expect("import json");
    assert_eq!(import_json["imported"], json!(1));
    assert_eq!(import_json["rejected"], json!([]));

    let get_order = app
        .oneshot(
            Request::builder()
                .uri("/v2/orders/57883507-70c4-4a1b-87e8-325e32c88700")
                .header("apca-api-key-id", "mock-key")
                .header("apca-api-secret-key", "mock-secret")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("get order should succeed");
    assert_eq!(get_order.status(), StatusCode::OK);
    let order_body = to_bytes(get_order.into_body(), usize::MAX)
        .await
        .expect("order body should read");
    let order: Value = serde_json::from_slice(&order_body).expect("order json");
    assert_eq!(order["id"], json!("57883507-70c4-4a1b-87e8-325e32c88700"));
    assert_eq!(order["client_order_id"], json!("grid-submit-1"));
    assert_eq!(order["status"], json!("new"));
    assert_eq!(order["type"], json!("limit"));
    assert_eq!(order["time_in_force"], json!("gtc"));
    assert_eq!(order["extended_hours"], json!(true));
    assert_eq!(order["filled_qty"], json!("0"));
}
