#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::Duration;

use rust_decimal::Decimal;

use alpaca_data::Client as DataClient;
use alpaca_mock::{LiveMarketDataBridge, MockServerState, spawn_test_server_with_state};
use alpaca_trade::{
    Client,
    activities::ListRequest,
    orders::{CreateRequest, OrderSide, OrderType, TimeInForce},
    positions::ClosePositionRequest,
};

use live_support::{AlpacaService, LiveTestEnv};

#[tokio::test]
async fn activities_mock_list_tracks_order_and_position_lifecycle() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock activities test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client));
    let server = spawn_test_server_with_state(state).await;
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("phase14-mock-activity-open".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock open order should succeed");
    let _ = wait_for_activity_count(&client, 1).await;

    client
        .positions()
        .close("SPY", ClosePositionRequest::default())
        .await
        .expect("mock close position should succeed");

    let fills = client
        .activities()
        .list(ListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            ..ListRequest::default()
        })
        .await
        .expect("mock fill activities should load");
    assert!(fills.len() >= 2);
    assert!(
        fills
            .iter()
            .all(|activity| activity.activity_type == "FILL")
    );
    assert!(
        fills
            .iter()
            .all(|activity| activity.symbol.as_deref() == Some("SPY"))
    );

    let all = client
        .activities()
        .list(ListRequest::default())
        .await
        .expect("mock activities list should load");
    assert!(!all.is_empty());
}

async fn wait_for_activity_count(client: &Client, minimum: usize) -> usize {
    for _attempt in 0..10 {
        let activities = client
            .activities()
            .list(ListRequest::default())
            .await
            .expect("mock activities list should remain readable");
        if activities.len() >= minimum {
            return activities.len();
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    client
        .activities()
        .list(ListRequest::default())
        .await
        .expect("mock activities list should remain readable")
        .len()
}
