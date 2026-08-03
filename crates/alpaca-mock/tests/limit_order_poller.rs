use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use alpaca_mock::LiveMarketDataBridge;
use alpaca_mock::state::{
    CreateOrderInput, ListActivitiesFilter, ListOrdersFilter, MockServerState,
};
use alpaca_trade::orders::{
    OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
    QueryOrderStatus, TimeInForce,
};
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

const API_KEY: &str = "limit-poller-key";
const SECOND_API_KEY: &str = "limit-poller-second-key";
const SINGLE_CALL: &str = "IWM261218C00220000";
const LONG_CALL: &str = "IWM261218C00200000";
const SHORT_CALL: &str = "IWM261218C00210000";
const MISSING_LONG_PUT: &str = "IWM261218P00190000";
const MISSING_SHORT_PUT: &str = "IWM261218P00180000";

#[tokio::test]
async fn limit_order_poll_batches_market_data_and_fills_supported_orders_once() {
    let market = MarketDataServer::spawn().await;
    let client = alpaca_data::Client::builder()
        .api_key("market-data-key")
        .secret_key("market-data-secret")
        .base_url_str(&market.base_url)
        .expect("loopback market-data URL should be valid")
        .build()
        .expect("market-data client should build");
    let state = MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(client));

    let empty_report = state.poll_limit_orders_once().await;
    assert!(empty_report.filled_order_ids.is_empty());
    assert_eq!(market.stock_batch_calls.load(Ordering::SeqCst), 0);
    assert_eq!(market.option_batch_calls.load(Ordering::SeqCst), 0);

    let stock = create_simple_limit(
        &state,
        API_KEY,
        "SPY",
        OrderSide::Buy,
        Decimal::new(100, 0),
        TimeInForce::Day,
        "poll-stock",
    )
    .await;
    let untouched_stock = create_simple_limit(
        &state,
        API_KEY,
        "QQQ",
        OrderSide::Buy,
        Decimal::new(100, 0),
        TimeInForce::Gtc,
        "poll-untouched-stock",
    )
    .await;
    let option = create_simple_limit(
        &state,
        API_KEY,
        SINGLE_CALL,
        OrderSide::Buy,
        Decimal::new(2, 0),
        TimeInForce::Gtc,
        "poll-option",
    )
    .await;
    let unsupported_ioc = create_simple_limit(
        &state,
        API_KEY,
        SINGLE_CALL,
        OrderSide::Buy,
        Decimal::new(2, 0),
        TimeInForce::Ioc,
        "poll-ioc",
    )
    .await;
    let second_account_stock = create_simple_limit(
        &state,
        SECOND_API_KEY,
        "SPY",
        OrderSide::Buy,
        Decimal::new(100, 0),
        TimeInForce::Gtc,
        "poll-second-account-stock",
    )
    .await;
    let mleg = state
        .create_order(
            API_KEY,
            mleg_limit_input(LONG_CALL, SHORT_CALL, Decimal::new(30, 2), "poll-mleg"),
        )
        .await
        .expect("resting MLEG should be created");
    let incomplete_mleg = state
        .create_order(
            API_KEY,
            mleg_limit_input(
                MISSING_LONG_PUT,
                MISSING_SHORT_PUT,
                Decimal::new(30, 2),
                "poll-incomplete-mleg",
            ),
        )
        .await
        .expect("resting MLEG with initially complete quotes should be created");

    for order in [
        &stock,
        &untouched_stock,
        &option,
        &unsupported_ioc,
        &second_account_stock,
        &mleg,
        &incomplete_mleg,
    ] {
        assert_eq!(order.status, OrderStatus::New);
        assert_eq!(order.filled_qty, Decimal::ZERO);
    }

    market.stock_batch_calls.store(0, Ordering::SeqCst);
    market.option_batch_calls.store(0, Ordering::SeqCst);
    market
        .stock_requested_symbols
        .lock()
        .expect("stock request log should not poison")
        .clear();
    market
        .option_requested_symbols
        .lock()
        .expect("option request log should not poison")
        .clear();
    market.phase.store(1, Ordering::SeqCst);

    let report = state.poll_limit_orders_once().await;
    assert_eq!(report.stock_market_data_error, None);
    assert_eq!(report.option_market_data_error, None);
    let mut expected_filled = vec![
        stock.id.clone(),
        second_account_stock.id.clone(),
        option.id.clone(),
        mleg.id.clone(),
    ];
    expected_filled.sort();
    assert_eq!(report.filled_order_ids, expected_filled);
    assert_eq!(market.stock_batch_calls.load(Ordering::SeqCst), 1);
    assert_eq!(market.option_batch_calls.load(Ordering::SeqCst), 1);
    let stock_requests = market
        .stock_requested_symbols
        .lock()
        .expect("stock request log should not poison");
    assert_eq!(
        stock_requests.as_slice(),
        &[vec!["QQQ".to_owned(), "SPY".to_owned()]]
    );
    drop(stock_requests);
    let mut expected_option_symbols = vec![
        SINGLE_CALL.to_owned(),
        LONG_CALL.to_owned(),
        SHORT_CALL.to_owned(),
        MISSING_LONG_PUT.to_owned(),
        MISSING_SHORT_PUT.to_owned(),
    ];
    expected_option_symbols.sort();
    let option_requests = market
        .option_requested_symbols
        .lock()
        .expect("option request log should not poison");
    assert_eq!(option_requests.as_slice(), &[expected_option_symbols]);
    drop(option_requests);

    assert_filled(
        state
            .get_by_client_order_id(API_KEY, "poll-stock")
            .expect("stock order should be queryable by client ID"),
        Decimal::new(99, 0),
    );
    assert_filled(
        state
            .get_by_client_order_id(SECOND_API_KEY, "poll-second-account-stock")
            .expect("second account stock order should be queryable"),
        Decimal::new(99, 0),
    );
    assert_filled(
        state
            .get_order(API_KEY, &option.id, false)
            .expect("option order should be queryable by ID"),
        Decimal::new(190, 2),
    );
    let filled_mleg = state
        .get_order(API_KEY, &mleg.id, true)
        .expect("MLEG should be queryable with nested legs");
    assert_filled(filled_mleg.clone(), Decimal::new(20, 2));
    let legs = filled_mleg
        .legs
        .expect("filled MLEG should include nested legs");
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].status, OrderStatus::Filled);
    assert_eq!(legs[0].filled_avg_price, Some(Decimal::new(75, 2)));
    assert_eq!(legs[1].status, OrderStatus::Filled);
    assert_eq!(legs[1].filled_avg_price, Some(Decimal::new(55, 2)));
    assert!(
        legs.iter()
            .all(|leg| leg.filled_at == filled_mleg.filled_at)
    );

    for order in [&untouched_stock, &unsupported_ioc, &incomplete_mleg] {
        let current = state
            .get_order(API_KEY, &order.id, true)
            .expect("resting order should remain queryable");
        assert_eq!(current.status, OrderStatus::New);
        assert_eq!(current.filled_qty, Decimal::ZERO);
    }

    let listed = state.list_orders(
        API_KEY,
        ListOrdersFilter {
            status: Some(QueryOrderStatus::All),
            nested: Some(true),
            ..ListOrdersFilter::default()
        },
    );
    assert!(listed.iter().any(|order| {
        order.id == mleg.id
            && order
                .legs
                .as_ref()
                .is_some_and(|legs| legs.iter().all(|leg| leg.status == OrderStatus::Filled))
    }));

    let account_after_first = state.project_account(API_KEY);
    let positions_after_first = state
        .list_positions(API_KEY)
        .await
        .expect("positions should project after fills");
    let fills_after_first = fill_activities(&state, API_KEY);
    assert_eq!(fills_after_first.len(), 3);
    let second_account_fills_after_first = fill_activities(&state, SECOND_API_KEY);
    assert_eq!(second_account_fills_after_first.len(), 1);

    let repeated = state.poll_limit_orders_once().await;
    assert!(repeated.filled_order_ids.is_empty());
    assert_eq!(
        state.project_account(API_KEY).cash,
        account_after_first.cash
    );
    assert_eq!(
        state
            .list_positions(API_KEY)
            .await
            .expect("positions should remain queryable"),
        positions_after_first
    );
    assert_eq!(fill_activities(&state, API_KEY), fills_after_first);
    assert_eq!(
        fill_activities(&state, SECOND_API_KEY),
        second_account_fills_after_first
    );

    market.phase.store(0, Ordering::SeqCst);
    let canceled_during_fetch = create_simple_limit(
        &state,
        API_KEY,
        "DIA",
        OrderSide::Buy,
        Decimal::new(50, 0),
        TimeInForce::Day,
        "poll-cancel-race",
    )
    .await;
    assert_eq!(canceled_during_fetch.status, OrderStatus::New);
    market.phase.store(2, Ordering::SeqCst);
    let poll_state = state.clone();
    let mut in_flight_poll = tokio::spawn(async move { poll_state.poll_limit_orders_once().await });
    tokio::time::timeout(
        Duration::from_secs(5),
        market.poll_request_started.notified(),
    )
    .await
    .expect("poll should reach market data without holding the accounts lock");
    let cancel_state = state.clone();
    let canceled_order_id = canceled_during_fetch.id.clone();
    let mut cancel_task =
        tokio::task::spawn_blocking(move || cancel_state.cancel_order(API_KEY, &canceled_order_id));
    let cancel_result = match tokio::time::timeout(Duration::from_secs(5), &mut cancel_task).await {
        Ok(result) => result.expect("cancel task should finish"),
        Err(_) => {
            market.poll_request_release.notify_one();
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut in_flight_poll).await;
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut cancel_task).await;
            panic!("cancel blocked while the poll waited for market data");
        }
    };
    cancel_result.expect("cancel should win while the poll waits for market data");
    market.poll_request_release.notify_one();
    let race_report = match tokio::time::timeout(Duration::from_secs(5), &mut in_flight_poll).await
    {
        Ok(result) => result.expect("in-flight poll should finish"),
        Err(_) => {
            in_flight_poll.abort();
            panic!("poll did not finish after market data was released");
        }
    };
    assert!(
        !race_report
            .filled_order_ids
            .contains(&canceled_during_fetch.id)
    );
    assert_eq!(
        state
            .get_order(API_KEY, &canceled_during_fetch.id, false)
            .expect("canceled order should remain queryable")
            .status,
        OrderStatus::Canceled
    );
    assert_eq!(fill_activities(&state, API_KEY), fills_after_first);

    market.task.abort();
}

async fn create_simple_limit(
    state: &MockServerState,
    api_key: &str,
    symbol: &str,
    side: OrderSide,
    limit_price: Decimal,
    time_in_force: TimeInForce,
    client_order_id: &str,
) -> Order {
    state
        .create_order(
            api_key,
            CreateOrderInput {
                symbol: Some(symbol.to_owned()),
                qty: Some(Decimal::ONE),
                side: Some(side),
                order_type: Some(OrderType::Limit),
                time_in_force: Some(time_in_force),
                limit_price: Some(limit_price),
                client_order_id: Some(client_order_id.to_owned()),
                order_class: Some(OrderClass::Simple),
                ..CreateOrderInput::default()
            },
        )
        .await
        .expect("simple limit order should be created")
}

fn mleg_limit_input(
    long_symbol: &str,
    short_symbol: &str,
    limit_price: Decimal,
    client_order_id: &str,
) -> CreateOrderInput {
    CreateOrderInput {
        qty: Some(Decimal::ONE),
        order_type: Some(OrderType::Limit),
        time_in_force: Some(TimeInForce::Gtc),
        limit_price: Some(limit_price),
        client_order_id: Some(client_order_id.to_owned()),
        order_class: Some(OrderClass::Mleg),
        legs: Some(vec![
            OptionLegRequest {
                symbol: long_symbol.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Buy),
                position_intent: Some(PositionIntent::BuyToOpen),
            },
            OptionLegRequest {
                symbol: short_symbol.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Sell),
                position_intent: Some(PositionIntent::SellToOpen),
            },
        ]),
        ..CreateOrderInput::default()
    }
}

fn assert_filled(order: Order, expected_price: Decimal) {
    assert_eq!(order.status, OrderStatus::Filled);
    assert_eq!(order.filled_qty, Decimal::ONE);
    assert_eq!(order.filled_avg_price, Some(expected_price));
    assert!(order.filled_at.is_some());
    assert_eq!(
        order.updated_at,
        order.filled_at.expect("fill time should exist")
    );
}

fn fill_activities(
    state: &MockServerState,
    api_key: &str,
) -> Vec<alpaca_trade::activities::Activity> {
    state.list_activities(
        api_key,
        ListActivitiesFilter {
            activity_types: Some(vec!["FILL".to_owned()]),
            ..ListActivitiesFilter::default()
        },
    )
}

#[derive(Clone)]
struct MarketDataState {
    phase: Arc<AtomicUsize>,
    stock_batch_calls: Arc<AtomicUsize>,
    option_batch_calls: Arc<AtomicUsize>,
    stock_requested_symbols: Arc<Mutex<Vec<Vec<String>>>>,
    option_requested_symbols: Arc<Mutex<Vec<Vec<String>>>>,
    poll_request_started: Arc<Notify>,
    poll_request_release: Arc<Notify>,
}

struct MarketDataServer {
    base_url: String,
    phase: Arc<AtomicUsize>,
    stock_batch_calls: Arc<AtomicUsize>,
    option_batch_calls: Arc<AtomicUsize>,
    stock_requested_symbols: Arc<Mutex<Vec<Vec<String>>>>,
    option_requested_symbols: Arc<Mutex<Vec<Vec<String>>>>,
    poll_request_started: Arc<Notify>,
    poll_request_release: Arc<Notify>,
    task: JoinHandle<()>,
}

impl MarketDataServer {
    async fn spawn() -> Self {
        let state = MarketDataState {
            phase: Arc::new(AtomicUsize::new(0)),
            stock_batch_calls: Arc::new(AtomicUsize::new(0)),
            option_batch_calls: Arc::new(AtomicUsize::new(0)),
            stock_requested_symbols: Arc::new(Mutex::new(Vec::new())),
            option_requested_symbols: Arc::new(Mutex::new(Vec::new())),
            poll_request_started: Arc::new(Notify::new()),
            poll_request_release: Arc::new(Notify::new()),
        };
        let app = Router::new()
            .route("/v2/stocks/{symbol}/snapshot", get(stock_snapshot))
            .route("/v2/stocks/snapshots", get(stock_snapshots))
            .route("/v1beta1/options/snapshots", get(option_snapshots))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback market-data listener should bind");
        let address = listener
            .local_addr()
            .expect("listener address should exist");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("loopback market-data server should run");
        });

        Self {
            base_url: format!("http://{address}"),
            phase: state.phase,
            stock_batch_calls: state.stock_batch_calls,
            option_batch_calls: state.option_batch_calls,
            stock_requested_symbols: state.stock_requested_symbols,
            option_requested_symbols: state.option_requested_symbols,
            poll_request_started: state.poll_request_started,
            poll_request_release: state.poll_request_release,
            task,
        }
    }
}

async fn stock_snapshot(
    Path(symbol): Path<String>,
    State(state): State<MarketDataState>,
) -> Json<Value> {
    let phase = state.phase.load(Ordering::SeqCst);
    let mut snapshot = stock_snapshot_value(&symbol, phase);
    snapshot
        .as_object_mut()
        .expect("snapshot fixture should be an object")
        .insert("symbol".to_owned(), json!(symbol.to_ascii_uppercase()));
    Json(snapshot)
}

#[derive(Debug, Deserialize)]
struct SnapshotQuery {
    symbols: String,
}

async fn stock_snapshots(
    Query(query): Query<SnapshotQuery>,
    State(state): State<MarketDataState>,
) -> Json<Value> {
    state.stock_batch_calls.fetch_add(1, Ordering::SeqCst);
    state
        .stock_requested_symbols
        .lock()
        .expect("stock request log should not poison")
        .push(normalized_query_symbols(&query.symbols));
    let phase = state.phase.load(Ordering::SeqCst);
    if phase == 2 {
        state.poll_request_started.notify_one();
        state.poll_request_release.notified().await;
    }

    let mut snapshots = Map::new();
    for symbol in ["SPY", "QQQ", "DIA"] {
        snapshots.insert(symbol.to_owned(), stock_snapshot_value(symbol, phase));
    }
    Json(Value::Object(snapshots))
}

async fn option_snapshots(
    Query(query): Query<SnapshotQuery>,
    State(state): State<MarketDataState>,
) -> Json<Value> {
    state.option_batch_calls.fetch_add(1, Ordering::SeqCst);
    state
        .option_requested_symbols
        .lock()
        .expect("option request log should not poison")
        .push(normalized_query_symbols(&query.symbols));
    let phase = state.phase.load(Ordering::SeqCst);
    let crossed = phase >= 1;
    let mut snapshots = Map::new();
    snapshots.insert(
        SINGLE_CALL.to_owned(),
        option_snapshot_value(
            if crossed { "1.80" } else { "2.00" },
            if crossed { "2.00" } else { "2.20" },
        ),
    );
    snapshots.insert(
        LONG_CALL.to_owned(),
        option_snapshot_value(
            if crossed { "0.70" } else { "1.00" },
            if crossed { "0.80" } else { "1.10" },
        ),
    );
    snapshots.insert(SHORT_CALL.to_owned(), option_snapshot_value("0.50", "0.60"));
    snapshots.insert(
        MISSING_LONG_PUT.to_owned(),
        option_snapshot_value("0.90", "1.00"),
    );
    if !crossed {
        snapshots.insert(
            MISSING_SHORT_PUT.to_owned(),
            option_snapshot_value("0.40", "0.50"),
        );
    }

    Json(json!({
        "snapshots": snapshots,
        "next_page_token": null
    }))
}

fn stock_snapshot_value(symbol: &str, phase: usize) -> Value {
    let (bid, ask) = match (symbol, phase) {
        ("SPY", 0) => ("100.00", "102.00"),
        ("SPY", _) => ("98.00", "100.00"),
        ("DIA", 2) => ("48.00", "50.00"),
        ("DIA", _) => ("50.00", "52.00"),
        _ => ("300.00", "302.00"),
    };
    json!({
        "latestQuote": { "bp": bid, "ap": ask },
        "prevDailyBar": { "c": bid }
    })
}

fn option_snapshot_value(bid: &str, ask: &str) -> Value {
    json!({
        "latestQuote": { "bp": bid, "ap": ask },
        "prevDailyBar": { "c": bid }
    })
}

fn normalized_query_symbols(symbols: &str) -> Vec<String> {
    let mut symbols = symbols
        .split(',')
        .map(|symbol| symbol.trim().to_ascii_uppercase())
        .filter(|symbol| !symbol.is_empty())
        .collect::<Vec<_>>();
    symbols.sort();
    symbols
}
