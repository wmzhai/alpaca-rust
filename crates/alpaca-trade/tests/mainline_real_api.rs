#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use serde_json::json;

use alpaca_trade::{
    Client, Error,
    activities::ListRequest as ActivitiesListRequest,
    orders::{
        CreateRequest, ListRequest as OrdersListRequest, OrderSide, OrderStatus, OrderType,
        QueryOrderStatus, SortDirection, TimeInForce,
    },
    positions::ClosePositionRequest,
};

use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, can_submit_live_paper_orders,
    paper_market_session_state, trading_day_from_timestamp,
};

const MAINLINE_SYMBOL: &str = "SPY";

#[tokio::test]
async fn trade_mainline_real_paper_flow_keeps_account_orders_positions_and_activities_in_sync() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let trade_service = env.trade().expect("trade config should exist");
    let trade_client = Client::builder()
        .credentials(trade_service.credentials().clone())
        .base_url(trade_service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let paper_state = paper_market_session_state(&probe, trade_service, Some(&recorder))
        .await
        .expect("paper clock and calendar should be readable");
    if !can_submit_live_paper_orders(&paper_state) {
        eprintln!("skipping paper mainline test: market session is unavailable");
        return;
    }
    let trading_day = trading_day_from_timestamp(&paper_state.clock.timestamp)
        .expect("paper clock timestamp should contain a trading day");

    ensure_symbol_flat(&trade_client, MAINLINE_SYMBOL).await;

    let account_before = trade_client
        .account()
        .get()
        .await
        .expect("real paper account should be readable before the lifecycle starts");

    let client_order_id = format!(
        "phase15-paper-mainline-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );

    let opened = trade_client
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
        .expect("real paper open order should submit");
    let opened = wait_for_order_status(&trade_client, &opened.id, OrderStatus::Filled).await;

    let opened_position = wait_for_position(&trade_client, MAINLINE_SYMBOL).await;
    assert_eq!(opened_position.symbol, MAINLINE_SYMBOL);

    let fills_after_open =
        wait_for_fill_activity(&trade_client, &opened.id, trading_day.clone()).await;
    assert!(
        fills_after_open
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&opened.id))
    );

    let close = trade_client
        .positions()
        .close(MAINLINE_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("real paper close position should submit");
    let closed = wait_for_order_status(&trade_client, &close.id, OrderStatus::Filled).await;
    wait_for_position_absent(&trade_client, MAINLINE_SYMBOL).await;

    let fills_after_close = wait_for_fill_activity(&trade_client, &closed.id, trading_day).await;
    assert!(
        fills_after_close
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&closed.id))
    );

    let orders = trade_client
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(50),
            ..OrdersListRequest::default()
        })
        .await
        .expect("real paper orders list should expose the full mainline lifecycle");
    assert!(orders.iter().any(|order| order.id == opened.id));
    assert!(orders.iter().any(|order| order.id == closed.id));

    let account_after = trade_client
        .account()
        .get()
        .await
        .expect("real paper account should remain readable after the lifecycle");
    assert_eq!(account_before.id, account_after.id);
    assert!(account_before.cash.is_some());
    assert!(account_after.cash.is_some());

    recorder
        .record_json(
            "alpaca-trade-mainline",
            "paper-lifecycle",
            &json!({
                "account_before": account_before,
                "open_order": opened,
                "open_position": opened_position,
                "close_order": closed,
                "fills_after_close": fills_after_close,
                "account_after": account_after,
            }),
        )
        .expect("paper mainline lifecycle sample should record");
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

async fn wait_for_fill_activity(
    client: &Client,
    order_id: &str,
    trading_day: String,
) -> Vec<alpaca_trade::activities::Activity> {
    for _attempt in 0..60 {
        let fills = client
            .activities()
            .list(ActivitiesListRequest {
                activity_types: Some(vec!["FILL".to_owned()]),
                date: Some(trading_day.clone()),
                direction: Some(SortDirection::Desc),
                page_size: Some(100),
                ..ActivitiesListRequest::default()
            })
            .await
            .expect("real paper fill activities should stay readable");
        if fills
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(order_id))
        {
            return fills;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client
        .activities()
        .list(ActivitiesListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            date: Some(trading_day),
            direction: Some(SortDirection::Desc),
            page_size: Some(100),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("real paper fill activities should remain readable")
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
        other => panic!("position {symbol} should disappear after the close order, got {other:?}"),
    }
}
