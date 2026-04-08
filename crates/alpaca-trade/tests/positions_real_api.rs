#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;

use alpaca_data::Client as DataClient;
use alpaca_trade::{
    Client, Error,
    orders::{CreateRequest, OrderSide, OrderStatus, OrderType, TimeInForce},
    positions::ClosePositionRequest,
};

use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, can_submit_live_paper_orders,
    paper_market_session_state,
};

const POSITION_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn positions_resource_reads_real_paper_positions_after_open_and_close() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let trade_service = env.trade().expect("trade config should exist");
    let data_service = env.data().expect("data config should exist");
    let trade_client = Client::builder()
        .credentials(trade_service.credentials().clone())
        .base_url(trade_service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let _data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let paper_state = paper_market_session_state(&probe, trade_service, Some(&recorder))
        .await
        .expect("paper clock and calendar should be readable");
    if !can_submit_live_paper_orders(&paper_state) {
        eprintln!("skipping paper positions test: market session is unavailable");
        return;
    }

    ensure_symbol_flat(&trade_client, POSITION_TEST_SYMBOL).await;

    let client_order_id = format!(
        "phase13-paper-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );

    let opened = trade_client
        .orders()
        .create(CreateRequest {
            symbol: Some(POSITION_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(client_order_id),
            ..CreateRequest::default()
        })
        .await
        .expect("real paper open order should submit");
    let opened = wait_for_order_status(&trade_client, &opened.id, OrderStatus::Filled).await;

    let position = wait_for_position(&trade_client, POSITION_TEST_SYMBOL).await;
    recorder
        .record_json("alpaca-trade-positions", "paper-open-position", &position)
        .expect("opened position sample should record");

    let listed = trade_client
        .positions()
        .list()
        .await
        .expect("positions list should succeed against real paper API");
    assert!(listed.iter().any(|candidate| {
        candidate.symbol == POSITION_TEST_SYMBOL && candidate.asset_id == position.asset_id
    }));

    let by_symbol = trade_client
        .positions()
        .get(POSITION_TEST_SYMBOL)
        .await
        .expect("positions get by symbol should succeed");
    let by_asset_id = trade_client
        .positions()
        .get(&position.asset_id)
        .await
        .expect("positions get by asset id should succeed");
    assert_eq!(by_symbol.asset_id, by_asset_id.asset_id);

    let close = trade_client
        .positions()
        .close(POSITION_TEST_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit against real paper API");
    let closed = wait_for_order_status(&trade_client, &close.id, OrderStatus::Filled).await;
    recorder
        .record_json("alpaca-trade-positions", "paper-close-order", &closed)
        .expect("closed order sample should record");
    wait_for_position_absent(&trade_client, POSITION_TEST_SYMBOL).await;

    assert_eq!(opened.asset_id, closed.asset_id);
}

async fn wait_for_order_status(
    client: &Client,
    order_id: &str,
    expected_status: OrderStatus,
) -> alpaca_trade::orders::Order {
    for _attempt in 0..20 {
        let order = client
            .orders()
            .get(order_id)
            .await
            .expect("paper order should stay readable");
        if order.status == expected_status {
            return order;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client
        .orders()
        .get(order_id)
        .await
        .expect("paper order should remain readable")
}

async fn wait_for_position(client: &Client, symbol: &str) -> alpaca_trade::positions::Position {
    for _attempt in 0..20 {
        if let Ok(position) = client.positions().get(symbol).await {
            return position;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client
        .positions()
        .get(symbol)
        .await
        .expect("position should become readable")
}

async fn wait_for_position_absent(client: &Client, symbol: &str) {
    for _attempt in 0..20 {
        match client.positions().get(symbol).await {
            Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {
                return;
            }
            Err(other) => panic!("unexpected position lookup error: {other:?}"),
            Ok(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }

    match client.positions().get(symbol).await {
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {}
        other => panic!("position {symbol} should disappear after close, got {other:?}"),
    }
}

async fn ensure_symbol_flat(client: &Client, symbol: &str) {
    let existing = match client.positions().get(symbol).await {
        Ok(position) => Some(position),
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => None,
        Err(other) => panic!("unexpected preflight position lookup error: {other:?}"),
    };
    if existing.is_none() {
        return;
    }

    let close = client
        .positions()
        .close(symbol, ClosePositionRequest::default())
        .await
        .expect("preflight close position should submit");
    let _ = wait_for_order_status(client, &close.id, OrderStatus::Filled).await;
    wait_for_position_absent(client, symbol).await;
}
