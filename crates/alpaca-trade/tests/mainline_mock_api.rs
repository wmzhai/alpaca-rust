#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;

use alpaca_data::Client as DataClient;
use alpaca_mock::{LiveMarketDataBridge, MockServerState, spawn_test_server_with_state};
use alpaca_trade::{
    Client, Error,
    activities::ListRequest as ActivitiesListRequest,
    orders::{CreateRequest, OrderSide, OrderStatus, OrderType, QueryOrderStatus, TimeInForce},
    positions::ClosePositionRequest,
};

use live_support::{AlpacaService, LiveTestEnv};

const MAINLINE_SYMBOL: &str = "SPY";

#[tokio::test]
async fn trade_mainline_mock_flow_keeps_account_orders_positions_and_activities_in_sync() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock mainline test: {reason}");
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

    let account_before = client
        .account()
        .get()
        .await
        .expect("mock account should be readable before the lifecycle starts");

    let client_order_id = format!(
        "phase15-mock-mainline-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );
    let opened = client
        .orders()
        .create(CreateRequest {
            symbol: Some(MAINLINE_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(client_order_id),
            ..CreateRequest::default()
        })
        .await
        .expect("mock open order should succeed");
    assert_eq!(opened.status, OrderStatus::Filled);

    let position = client
        .positions()
        .get(MAINLINE_SYMBOL)
        .await
        .expect("mock mainline open position should be readable");
    assert_eq!(position.symbol, MAINLINE_SYMBOL);

    let fills_after_open = client
        .activities()
        .list(ActivitiesListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("mock fill activities should be readable after the open order");
    assert!(
        fills_after_open
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&opened.id))
    );

    let close = client
        .positions()
        .close(MAINLINE_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("mock close position should succeed");
    let closed = client
        .orders()
        .get(&close.id)
        .await
        .expect("mock close order should remain readable");
    assert_eq!(closed.status, OrderStatus::Filled);
    wait_for_position_absent(&client, MAINLINE_SYMBOL).await;

    let fills_after_close = client
        .activities()
        .list(ActivitiesListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("mock fill activities should remain readable after the close order");
    assert!(
        fills_after_close
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&closed.id))
    );

    let orders = client
        .orders()
        .list(alpaca_trade::orders::ListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(50),
            ..alpaca_trade::orders::ListRequest::default()
        })
        .await
        .expect("mock orders list should expose the full mainline lifecycle");
    assert!(orders.iter().any(|order| order.id == opened.id));
    assert!(orders.iter().any(|order| order.id == closed.id));

    let account_after = client
        .account()
        .get()
        .await
        .expect("mock account should stay readable after the lifecycle");
    assert_eq!(account_before.id, account_after.id);
    assert!(account_before.cash.is_some());
    assert!(account_after.cash.is_some());
}

async fn wait_for_position_absent(client: &Client, symbol: &str) {
    for _attempt in 0..10 {
        match client.positions().get(symbol).await {
            Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {
                return;
            }
            Err(other) => panic!("unexpected position lookup error: {other:?}"),
            Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(250)).await,
        }
    }

    match client.positions().get(symbol).await {
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {}
        other => panic!("position {symbol} should disappear after the close order, got {other:?}"),
    }
}
