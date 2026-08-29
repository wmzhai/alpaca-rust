use std::env;

use alpaca_mock::{DEFAULT_STOCK_SYMBOL, RejectedReplacementRaceFixture};
use alpaca_trade::Client;
use alpaca_trade::orders::{
    ListRequest, OrderSide, OrderStatus, QueryOrderStatus, ReplaceRequest, ReplaceResolution,
    SubmitOrderRequest, SubmitOrderStyle, TimeInForce, TransitionOrderPolicy, TransitionResolution,
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

#[tokio::test]
async fn rejected_replacement_does_not_hide_accepted_predecessor() {
    assert_rejected_replacement_keeps_predecessor(OrderStatus::Accepted, Decimal::ZERO).await;
}

#[tokio::test]
async fn rejected_replacement_does_not_hide_partially_filled_predecessor() {
    assert_rejected_replacement_keeps_predecessor(OrderStatus::PartiallyFilled, Decimal::ONE).await;
}

async fn assert_rejected_replacement_keeps_predecessor(
    predecessor_status: OrderStatus,
    filled_qty: Decimal,
) {
    let server = alpaca_mock::spawn_test_server().await;
    let api_key = format!("keep-predecessor-{}", predecessor_status.as_str());
    let fixture = reqwest::Client::new()
        .post(format!(
            "{}/admin/fixtures/rejected-replacement-race",
            server.base_url
        ))
        .json(&json!({
            "api_key": api_key,
            "predecessor_status": predecessor_status,
        }))
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
        .secret_key("keep-predecessor-secret")
        .base_url_str(&server.base_url)
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
            "rejected replacement must not win over {predecessor_status:?} predecessor: {:?}",
            resolved.order
        ),
    };
    assert_eq!(resolved.order.id, fixture.predecessor_order_id);
    assert_eq!(resolved.order.status, predecessor_status);
    assert_eq!(resolved.order.filled_qty, filled_qty);

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
        .expect("later sync should recover the original predecessor");
    assert_eq!(synchronized.id, fixture.predecessor_order_id);
    assert_eq!(synchronized.status, predecessor_status);
    assert_eq!(synchronized.filled_qty, filled_qty);
}

#[tokio::test]
async fn transition_retry_recovers_one_stable_client_order_and_effective_predecessor() {
    let server = alpaca_mock::spawn_test_server().await;
    let api_key = "transition-retry-key";
    let fixture = reqwest::Client::new()
        .post(format!(
            "{}/admin/fixtures/rejected-replacement-race",
            server.base_url
        ))
        .json(&json!({ "api_key": api_key }))
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
        .secret_key("transition-retry-secret")
        .base_url_str(&server.base_url)
        .expect("mock base URL should be accepted")
        .build()
        .expect("mock trading client should build");
    let stable_client_order_id = "transition-retry-successor";
    let request = SubmitOrderRequest::simple(
        DEFAULT_STOCK_SYMBOL,
        1,
        OrderSide::Buy,
        SubmitOrderStyle::Limit {
            limit_price: Decimal::new(110, 2),
        },
        Some(TimeInForce::Day),
        None,
    )
    .with_client_order_id(stable_client_order_id);

    for _ in 0..2 {
        let resolution = client
            .orders()
            .transition_resolved(
                &fixture.predecessor_order_id,
                request.clone(),
                TransitionOrderPolicy::Auto,
            )
            .await
            .expect("transition should resolve on both the request and recovery paths");
        let TransitionResolution::OriginalOrderTerminal(resolved) = resolution else {
            panic!("rejected successor must resolve to its filled predecessor");
        };
        assert_eq!(resolved.order.id, fixture.predecessor_order_id);
        assert_eq!(resolved.order.status, OrderStatus::Filled);
    }

    let successors = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            ..ListRequest::default()
        })
        .await
        .expect("mock orders should list")
        .into_iter()
        .filter(|order| order.client_order_id == stable_client_order_id)
        .collect::<Vec<_>>();
    assert_eq!(
        successors.len(),
        1,
        "retry must not create another successor"
    );
    assert_eq!(
        successors[0].replaces.as_deref(),
        Some(fixture.predecessor_order_id.as_str())
    );
}

fn required_env(name: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{name} must be configured"))
}
