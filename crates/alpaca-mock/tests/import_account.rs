use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::util::ServiceExt;

fn decimal_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        other => panic!("expected decimal, got {other}"),
    }
}

#[tokio::test]
async fn admin_import_account_keeps_cash_after_order_import() {
    let app = alpaca_mock::build_app();
    let import_account = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/accounts/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "api_key": "mock-key",
                        "status": "ACTIVE",
                        "cash": "1500.25",
                        "portfolio_value": "6400.25",
                        "equity": "6400.25",
                        "long_market_value": "5000.00",
                        "short_market_value": "100.00",
                        "buying_power": "4000.50",
                        "maintenance_margin": "250.00",
                        "options_buying_power": "1800.00",
                        "multiplier": "4"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("account import should succeed");
    assert_eq!(import_account.status(), StatusCode::OK);

    let account = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key")
                .header("apca-api-secret-key", "mock-secret")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("account request should succeed");
    assert_eq!(account.status(), StatusCode::OK);
    let account_body = to_bytes(account.into_body(), usize::MAX)
        .await
        .expect("account body should read");
    let account_json: Value = serde_json::from_slice(&account_body).expect("account json");
    assert_eq!(decimal_text(&account_json["cash"]), "1500.25");
    assert_eq!(decimal_text(&account_json["buying_power"]), "4000.50");
    assert_eq!(decimal_text(&account_json["portfolio_value"]), "6400.25");
    assert_eq!(decimal_text(&account_json["long_market_value"]), "5000.00");
    assert_eq!(decimal_text(&account_json["maintenance_margin"]), "250.00");
    assert_eq!(decimal_text(&account_json["options_buying_power"]), "1800.00");

    let import_order = app
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
                            "id": "57883507-70c4-4a1b-87e8-325e32c88711",
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
        .expect("order import should succeed");
    assert_eq!(import_order.status(), StatusCode::OK);

    let account_after = app
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-key")
                .header("apca-api-secret-key", "mock-secret")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("account request should succeed");
    assert_eq!(account_after.status(), StatusCode::OK);
    let after_body = to_bytes(account_after.into_body(), usize::MAX)
        .await
        .expect("account body should read");
    let after_json: Value = serde_json::from_slice(&after_body).expect("account json");
    assert_eq!(decimal_text(&after_json["cash"]), "1500.25");
    assert_eq!(decimal_text(&after_json["buying_power"]), "4000.50");
}
