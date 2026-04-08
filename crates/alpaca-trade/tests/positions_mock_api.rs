#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;

use std::collections::BTreeSet;
use std::time::Duration;

use rust_decimal::Decimal;

use alpaca_data::Client as DataClient;
use alpaca_mock::{LiveMarketDataBridge, MockServerState, spawn_test_server_with_state};
use alpaca_trade::{
    Client, Error,
    orders::{CreateRequest, OrderClass, OrderSide, OrderStatus, OrderType, TimeInForce},
    positions::{CloseAllRequest, ClosePositionRequest},
};

use live_support::{AlpacaService, LiveTestEnv};
use order_support::discover_mleg_call_spread;

const SYMBOL_A: &str = "SPY";
const SYMBOL_B: &str = "QQQ";

#[tokio::test]
async fn positions_mock_supports_list_get_close_and_close_all() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock positions test: {reason}");
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

    let opened_spy = open_mock_position(&client, SYMBOL_A, "phase13-mock-spy-open").await;
    let opened_qqq = open_mock_position(&client, SYMBOL_B, "phase13-mock-qqq-open").await;

    let listed = client
        .positions()
        .list()
        .await
        .expect("mock positions list should succeed");
    let listed_symbols = listed
        .iter()
        .map(|position| position.symbol.clone())
        .collect::<BTreeSet<_>>();
    assert!(listed_symbols.contains(SYMBOL_A));
    assert!(listed_symbols.contains(SYMBOL_B));

    let by_symbol = client
        .positions()
        .get(SYMBOL_A)
        .await
        .expect("mock position get by symbol should succeed");
    let by_asset_id = client
        .positions()
        .get(&opened_spy.asset_id)
        .await
        .expect("mock position get by asset id should succeed");
    assert_eq!(by_symbol.asset_id, by_asset_id.asset_id);

    let close_spy = client
        .positions()
        .close(SYMBOL_A, ClosePositionRequest::default())
        .await
        .expect("mock close position should succeed");
    let closed_spy = client
        .orders()
        .get(&close_spy.id)
        .await
        .expect("mock close order should be readable");
    assert_eq!(closed_spy.status, OrderStatus::Filled);
    wait_for_position_absent(&client, SYMBOL_A).await;

    let close_results = client
        .positions()
        .close_all(CloseAllRequest::default())
        .await
        .expect("mock close_all should succeed");
    let close_all_symbols = close_results
        .iter()
        .map(|result| result.symbol.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(close_all_symbols, BTreeSet::from([SYMBOL_B.to_owned()]));

    let closed_qqq = client
        .orders()
        .get(
            &close_results[0]
                .body
                .as_ref()
                .expect("mock close_all body should be present")
                .id,
        )
        .await
        .expect("mock close_all order should be readable");
    assert_eq!(closed_qqq.status, OrderStatus::Filled);
    wait_for_position_absent(&client, SYMBOL_B).await;

    assert_ne!(opened_spy.asset_id, opened_qqq.asset_id);
}

#[tokio::test]
async fn positions_mock_project_filled_mleg_legs_without_creating_parent_combo_position() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock mleg positions test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let spread = discover_mleg_call_spread(&data_client, SYMBOL_A)
        .await
        .expect("dynamic call spread should be discoverable");
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

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(spread.marketable_limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some("phase20-mock-positions-mleg".to_owned()),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("mock marketable mleg order should succeed");
    assert_eq!(created.status, OrderStatus::Filled);

    let positions = client
        .positions()
        .list()
        .await
        .expect("mock positions list should succeed after mleg fill");
    assert_eq!(positions.len(), spread.legs.len());
    assert!(positions.iter().all(|position| !position.symbol.is_empty()));
    assert!(
        positions
            .iter()
            .all(|position| position.symbol != created.symbol)
    );
    for leg in &spread.legs {
        let position = positions
            .iter()
            .find(|position| position.symbol == leg.symbol)
            .expect("each spread leg should project into a position");
        assert_eq!(position.asset_class, "us_option");
        assert_eq!(position.qty, Decimal::ONE);
        assert_eq!(
            position.side,
            match leg.side {
                Some(OrderSide::Buy) => "long",
                Some(OrderSide::Sell) => "short",
                _ => panic!("spread leg should have a side"),
            }
        );
    }
}

async fn open_mock_position(
    client: &Client,
    symbol: &str,
    client_order_id: &str,
) -> alpaca_trade::positions::Position {
    let order = client
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(client_order_id.to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock open order should succeed");
    assert_eq!(order.status, OrderStatus::Filled);

    wait_for_position(client, symbol).await
}

async fn wait_for_position(client: &Client, symbol: &str) -> alpaca_trade::positions::Position {
    for _attempt in 0..10 {
        if let Ok(position) = client.positions().get(symbol).await {
            return position;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    client
        .positions()
        .get(symbol)
        .await
        .expect("position should become readable")
}

async fn wait_for_position_absent(client: &Client, symbol: &str) {
    for _attempt in 0..10 {
        match client.positions().get(symbol).await {
            Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {
                return;
            }
            Err(other) => panic!("unexpected position lookup error: {other:?}"),
            Ok(_) => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }

    match client.positions().get(symbol).await {
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {}
        other => panic!("position {symbol} should disappear after close, got {other:?}"),
    }
}
