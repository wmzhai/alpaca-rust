#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alpaca_data::{
    Client as DataClient,
    stocks::{DataFeed, SnapshotRequest},
};
use alpaca_trade::{
    Client,
    orders::{
        CreateRequest, ListRequest, OrderSide, OrderStatus, OrderType, QueryOrderStatus,
        TimeInForce,
    },
};
use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, can_submit_live_paper_orders,
    paper_market_session_state,
};
use rust_decimal::Decimal;

const ORDER_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn orders_resource_reads_real_paper_api_and_optionally_submits_order() {
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
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);

    let listed = trade_client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(20),
            ..ListRequest::default()
        })
        .await
        .expect("orders list should read from real paper API");
    recorder
        .record_json("alpaca-trade-orders", "paper-list", &listed)
        .expect("orders list sample should record");

    let probe = LiveHttpProbe::new().expect("live probe should build");
    let paper_state = paper_market_session_state(&probe, trade_service, Some(&recorder))
        .await
        .expect("paper clock and calendar should be readable");
    if !can_submit_live_paper_orders(&paper_state) {
        eprintln!("skipping paper order submission: market session is unavailable");
        return;
    }

    let snapshot = data_client
        .stocks()
        .snapshot(SnapshotRequest {
            symbol: ORDER_TEST_SYMBOL.to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("stock snapshot should load for real order pricing");
    let quote = snapshot
        .latest_quote
        .expect("stock snapshot should include latest quote");
    let bid = quote.bp.expect("latest quote should include bid price");
    let non_marketable_buy_price = (bid * Decimal::new(95, 2)).round_dp(2);
    let client_order_id = format!(
        "phase12-paper-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );

    let created = trade_client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
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
        .expect("real paper order create should succeed");
    recorder
        .record_json("alpaca-trade-orders", "paper-create", &created)
        .expect("created order sample should record");
    assert_eq!(created.symbol, ORDER_TEST_SYMBOL);

    let fetched = trade_client
        .orders()
        .get(&created.id)
        .await
        .expect("created paper order should be readable");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = trade_client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("paper client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    trade_client
        .orders()
        .cancel(&created.id)
        .await
        .expect("paper order cancel should succeed");

    let canceled = wait_for_order_status(&trade_client, &created.id, OrderStatus::Canceled)
        .await
        .expect("paper order should become canceled");
    recorder
        .record_json("alpaca-trade-orders", "paper-canceled", &canceled)
        .expect("canceled order sample should record");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}

async fn wait_for_order_status(
    client: &Client,
    order_id: &str,
    expected_status: OrderStatus,
) -> Result<alpaca_trade::orders::Order, alpaca_trade::Error> {
    for _attempt in 0..10 {
        let order = client.orders().get(order_id).await?;
        if order.status == expected_status {
            return Ok(order);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client.orders().get(order_id).await
}
