#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_data::Client as DataClient;
use alpaca_mock::{
    DEFAULT_STOCK_SYMBOL, LiveMarketDataBridge, MockServerState, spawn_test_server_with_state,
};
use alpaca_trade::{
    Client,
    orders::{
        CreateRequest, ListRequest, OrderSide, OrderStatus, OrderType, QueryOrderStatus,
        TimeInForce,
    },
};
use rust_decimal::Decimal;

use live_support::{AlpacaService, LiveTestEnv};

#[tokio::test]
async fn orders_mock_supports_basic_lifecycle() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock orders test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let market_bridge = LiveMarketDataBridge::new(data_client);
    let price_context = market_bridge
        .equity_snapshot(DEFAULT_STOCK_SYMBOL)
        .await
        .expect("equity snapshot should load for mock pricing");
    let non_marketable_buy_price = (price_context.bid * Decimal::new(95, 2)).round_dp(2);
    let state = MockServerState::new().with_market_data_bridge(market_bridge);
    let server = spawn_test_server_with_state(state).await;
    let client_order_id = format!(
        "phase12-mock-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(DEFAULT_STOCK_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(non_marketable_buy_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock order create should succeed");
    assert_eq!(created.symbol, DEFAULT_STOCK_SYMBOL);
    assert_eq!(created.status, OrderStatus::New);

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock order get should succeed");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("mock client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let open_orders = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            ..ListRequest::default()
        })
        .await
        .expect("mock order list should succeed");
    assert!(open_orders.iter().any(|order| order.id == created.id));

    client
        .orders()
        .cancel(&created.id)
        .await
        .expect("mock order cancel should succeed");

    let canceled = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock canceled order should still be readable");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}
