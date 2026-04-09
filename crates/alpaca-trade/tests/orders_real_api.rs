#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;

use std::{
    sync::OnceLock,
    time::Duration,
};

use alpaca_data::{
    Client as DataClient,
};
use alpaca_trade::{
    Client,
    orders::{
        CreateRequest, ListRequest, OrderSide, OrderStatus, OrderType, QueryOrderStatus,
        ReplaceRequest, TimeInForce,
    },
};
use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, can_submit_live_paper_orders,
    paper_market_session_state,
};
use order_support::{
    discover_mleg_call_spread, non_marketable_buy_limit_price, unique_client_order_id,
};
use rust_decimal::Decimal;
use tokio::sync::Mutex;

const ORDER_TEST_SYMBOL: &str = "SPY";
static ORDERS_REAL_API_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[tokio::test]
async fn orders_resource_reads_real_paper_api_and_optionally_submits_order() {
    let _guard = orders_real_api_lock().await;
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

    let non_marketable_buy_price = non_marketable_buy_limit_price(&data_client, ORDER_TEST_SYMBOL)
        .await
        .expect("non-marketable stock price should be discoverable from live market data");
    let client_order_id = unique_client_order_id("phase12-paper");

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

#[tokio::test]
async fn orders_replace_real_paper_mleg_limit_order_and_cancel_replacement() {
    let _guard = orders_real_api_lock().await;
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
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let paper_state = paper_market_session_state(&probe, trade_service, Some(&recorder))
        .await
        .expect("paper clock and calendar should be readable");
    if !can_submit_live_paper_orders(&paper_state) {
        eprintln!("skipping paper order replacement: market session is unavailable");
        return;
    }

    let spread = discover_mleg_call_spread(&data_client, ORDER_TEST_SYMBOL)
        .await
        .expect("quoted multi-leg call spread should be discoverable from the live option chain");
    let client_order_id = unique_client_order_id("phase20-paper-mleg");
    let replaced_client_order_id = format!("{client_order_id}-replaced");

    let created = trade_client
        .orders()
        .create(CreateRequest {
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(spread.non_marketable_limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            order_class: Some(alpaca_trade::orders::OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
            symbol: None,
        })
        .await
        .expect("real paper multi-leg order create should succeed");
    recorder
        .record_json("alpaca-trade-orders", "paper-mleg-create", &created)
        .expect("multi-leg created order sample should record");
    assert_eq!(created.order_class, alpaca_trade::orders::OrderClass::Mleg);
    assert!(created.legs.as_ref().is_some_and(|legs| legs.len() == 2));

    let fetched = trade_client
        .orders()
        .get(&created.id)
        .await
        .expect("created multi-leg paper order should be readable");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = trade_client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("multi-leg client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let replacement = trade_client
        .orders()
        .replace(
            &created.id,
            ReplaceRequest {
                qty: None,
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(spread.more_conservative_limit_price),
                stop_price: None,
                trail: None,
                client_order_id: Some(replaced_client_order_id.clone()),
            },
        )
        .await
        .expect("real paper multi-leg order replace should succeed");
    recorder
        .record_json("alpaca-trade-orders", "paper-mleg-replaced", &replacement)
        .expect("multi-leg replacement sample should record");
    assert_ne!(replacement.id, created.id);
    assert_eq!(replacement.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(
        replacement.limit_price,
        Some(spread.more_conservative_limit_price)
    );
    assert_eq!(
        replacement.order_class,
        alpaca_trade::orders::OrderClass::Mleg
    );

    let replaced_source = wait_for_order_status(&trade_client, &created.id, OrderStatus::Replaced)
        .await
        .expect("original multi-leg order should become replaced");
    assert_eq!(
        replaced_source.replaced_by.as_deref(),
        Some(replacement.id.as_str())
    );

    let replacement_by_client_order_id = trade_client
        .orders()
        .get_by_client_order_id(&replaced_client_order_id)
        .await
        .expect("replacement client_order_id lookup should succeed");
    assert_eq!(replacement_by_client_order_id.id, replacement.id);

    trade_client
        .orders()
        .cancel(&replacement.id)
        .await
        .expect("real paper replacement order cancel should succeed");

    let canceled = wait_for_order_status(&trade_client, &replacement.id, OrderStatus::Canceled)
        .await
        .expect("replacement order should become canceled");
    recorder
        .record_json("alpaca-trade-orders", "paper-mleg-canceled", &canceled)
        .expect("multi-leg canceled order sample should record");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}

#[tokio::test]
async fn orders_cancel_all_real_paper_cancels_open_limit_orders() {
    let _guard = orders_real_api_lock().await;
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
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let paper_state = paper_market_session_state(&probe, trade_service, Some(&recorder))
        .await
        .expect("paper clock and calendar should be readable");
    if !can_submit_live_paper_orders(&paper_state) {
        eprintln!("skipping paper cancel_all: market session is unavailable");
        return;
    }

    let non_marketable_buy_price = non_marketable_buy_limit_price(&data_client, ORDER_TEST_SYMBOL)
        .await
        .expect("non-marketable stock price should be discoverable for cancel_all");

    let mut created_ids = Vec::new();
    for index in 0..2 {
        let client_order_id = unique_client_order_id(&format!("phase20-paper-cancel-all-{index}"));
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
                client_order_id: Some(client_order_id),
                ..CreateRequest::default()
            })
            .await
            .expect("real paper order create should succeed before cancel_all");
        created_ids.push(created.id);
    }

    let canceled = trade_client
        .orders()
        .cancel_all()
        .await
        .expect("real paper cancel_all should succeed");
    recorder
        .record_json("alpaca-trade-orders", "paper-cancel-all", &canceled)
        .expect("cancel_all sample should record");
    assert!(
        created_ids
            .iter()
            .all(|id| canceled.iter().any(|result| &result.id == id))
    );

    for created_id in created_ids {
        let order = wait_for_order_status(&trade_client, &created_id, OrderStatus::Canceled)
            .await
            .expect("cancel_all should drive each created order to canceled");
        assert_eq!(order.status, OrderStatus::Canceled);
    }
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

async fn orders_real_api_lock() -> tokio::sync::MutexGuard<'static, ()> {
    ORDERS_REAL_API_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}
