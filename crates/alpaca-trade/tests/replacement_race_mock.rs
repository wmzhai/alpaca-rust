use std::env;

use alpaca_mock::RejectedReplacementRaceFixture;
use alpaca_trade::Client;
use alpaca_trade::orders::{
    ListRequest, OrderStatus, QueryOrderStatus, ReplaceRequest, ReplaceResolution, TimeInForce,
};
use rust_decimal::Decimal;
use serde_json::json;

const TARGET_ENV: &str = "T127_TRADING_TARGET";

#[tokio::test]
async fn rejected_replacement_does_not_hide_filled_predecessor() {
    assert_eq!(required_env(TARGET_ENV), "mock");
    let base_url = required_env(alpaca_trade::TRADE_BASE_URL_ENV);
    let parsed = reqwest::Url::parse(&base_url).expect("mock base URL should be valid");
    assert_eq!(parsed.scheme(), "http");
    assert!(matches!(
        parsed.host_str(),
        Some("127.0.0.1") | Some("localhost")
    ));

    let api_key = required_env(alpaca_trade::TRADE_API_KEY_ENV);
    let fixture = reqwest::Client::new()
        .post(format!(
            "{base_url}/admin/fixtures/rejected-replacement-race"
        ))
        .json(&json!({ "api_key": api_key.clone() }))
        .send()
        .await
        .expect("replacement race fixture request should succeed")
        .error_for_status()
        .expect("replacement race fixture should return success")
        .json::<RejectedReplacementRaceFixture>()
        .await
        .expect("replacement race fixture should deserialize");

    let client = Client::builder()
        .api_key(api_key)
        .secret_key(required_env(alpaca_trade::TRADE_SECRET_KEY_ENV))
        .base_url_str(&base_url)
        .expect("mock base URL should be accepted")
        .build()
        .expect("mock trading client should build");

    let resolution = client
        .orders()
        .replace_resolved(
            &fixture.predecessor_order_id,
            ReplaceRequest {
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(110, 2)),
                ..ReplaceRequest::default()
            },
        )
        .await
        .expect("replacement race should resolve");

    let resolved = match resolution {
        ReplaceResolution::OriginalOrderTerminal(resolved) => resolved,
        ReplaceResolution::NewOrder(resolved) => panic!(
            "rejected replacement must not win over filled predecessor: {:?}",
            resolved.order
        ),
    };
    assert_eq!(resolved.order.id, fixture.predecessor_order_id);
    assert_eq!(resolved.order.status, OrderStatus::Filled);

    let replacement_id = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            ..ListRequest::default()
        })
        .await
        .expect("mock orders should list")
        .into_iter()
        .find(|order| order.replaces.as_deref() == Some(fixture.predecessor_order_id.as_str()))
        .expect("rejected replacement should remain queryable")
        .id;
    let synchronized = client
        .orders()
        .get_effective(&replacement_id)
        .await
        .expect("later sync should recover filled predecessor");
    assert_eq!(synchronized.id, fixture.predecessor_order_id);
    assert_eq!(synchronized.status, OrderStatus::Filled);
}

fn required_env(name: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{name} must be configured"))
}
