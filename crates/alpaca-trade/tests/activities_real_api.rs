#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;

use alpaca_trade::{
    Client,
    activities::{ListByTypeRequest, ListRequest},
    orders::{CreateRequest, OrderSide, OrderStatus, OrderType, TimeInForce},
    positions::ClosePositionRequest,
};

use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, can_submit_live_paper_orders,
    paper_market_session_state, trading_day_from_timestamp,
};

const ACTIVITY_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn activities_resource_reads_real_paper_specific_type_endpoint() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.trade().expect("trade config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .base_url(service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);
    let clock = client
        .clock()
        .get()
        .await
        .expect("clock request should succeed before the activities query");
    let trading_day = trading_day_from_timestamp(&clock.timestamp)
        .expect("paper clock timestamp should contain a trading day");

    let fills = client
        .activities()
        .list_by_type(
            "FILL",
            ListByTypeRequest {
                date: Some(trading_day),
                direction: Some(alpaca_trade::orders::SortDirection::Desc),
                page_size: Some(100),
                ..ListByTypeRequest::default()
            },
        )
        .await
        .expect("typed activities endpoint should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-activities", "list-by-type-fill", &fills)
        .expect("typed activities sample should record");

    assert!(fills.iter().all(|activity| activity.activity_type == "FILL"));
}

#[tokio::test]
async fn activities_resource_reads_real_paper_fill_activities_after_open_and_close() {
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
        eprintln!("skipping paper activities test: market session is unavailable");
        return;
    }
    let trading_day = trading_day_from_timestamp(&paper_state.clock.timestamp)
        .expect("paper clock timestamp should contain a trading day");

    ensure_symbol_flat(&trade_client, ACTIVITY_TEST_SYMBOL).await;

    let client_order_id = format!(
        "phase14-paper-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );

    let opened = trade_client
        .orders()
        .create(CreateRequest {
            symbol: Some(ACTIVITY_TEST_SYMBOL.to_owned()),
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
    let close = trade_client
        .positions()
        .close(ACTIVITY_TEST_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit against real paper API");
    let closed = wait_for_order_status(&trade_client, &close.id, OrderStatus::Filled).await;

    let fills = wait_for_fill_activities(
        &trade_client,
        &[opened.id.as_str(), closed.id.as_str()],
        trading_day,
    )
    .await;
    recorder
        .record_json("alpaca-trade-activities", "paper-fills", &fills)
        .expect("fill activities sample should record");

    assert!(
        fills
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&opened.id))
    );
    assert!(
        fills
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&closed.id))
    );
    assert!(
        fills
            .iter()
            .all(|activity| activity.activity_type == "FILL")
    );
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

async fn wait_for_fill_activities(
    client: &Client,
    order_ids: &[&str],
    trading_day: String,
) -> Vec<alpaca_trade::activities::Activity> {
    for _attempt in 0..60 {
        let fills = client
            .activities()
            .list(ListRequest {
                activity_types: Some(vec!["FILL".to_owned()]),
                date: Some(trading_day.clone()),
                direction: Some(alpaca_trade::orders::SortDirection::Desc),
                page_size: Some(100),
                ..ListRequest::default()
            })
            .await
            .expect("real paper fill activities should stay readable");
        if order_ids.iter().all(|order_id| {
            fills
                .iter()
                .any(|activity| activity.order_id.as_deref() == Some(*order_id))
        }) {
            return fills;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client
        .activities()
        .list(ListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            date: Some(trading_day),
            direction: Some(alpaca_trade::orders::SortDirection::Desc),
            page_size: Some(100),
            ..ListRequest::default()
        })
        .await
        .expect("real paper fill activities should remain readable")
}

async fn ensure_symbol_flat(client: &Client, symbol: &str) {
    let existing = match client.positions().get(symbol).await {
        Ok(position) => Some(position),
        Err(alpaca_trade::Error::Http(error))
            if error.meta().map(|meta| meta.status()) == Some(404) =>
        {
            None
        }
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

async fn wait_for_position_absent(client: &Client, symbol: &str) {
    for _attempt in 0..20 {
        match client.positions().get(symbol).await {
            Err(alpaca_trade::Error::Http(error))
                if error.meta().map(|meta| meta.status()) == Some(404) =>
            {
                return;
            }
            Err(other) => panic!("unexpected position lookup error: {other:?}"),
            Ok(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }

    match client.positions().get(symbol).await {
        Err(alpaca_trade::Error::Http(error))
            if error.meta().map(|meta| meta.status()) == Some(404) => {}
        other => panic!("position {symbol} should disappear after cleanup, got {other:?}"),
    }
}
