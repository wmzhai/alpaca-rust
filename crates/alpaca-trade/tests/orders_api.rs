#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;

use std::{
    sync::OnceLock,
};

use alpaca_trade::{
    orders::{
        CreateRequest, ListRequest, OrderClass, OrderSide, OrderStatus, OrderType,
        QueryOrderStatus, ReplaceRequest, TimeInForce,
    },
    Error,
};
use live_support::can_submit_live_paper_orders;
use order_support::{
    clear_option_universe_cache, discover_mleg_call_spread, non_marketable_buy_limit_price,
    stock_order_price_context, unique_client_order_id,
};
use rust_decimal::Decimal;
use serde::Serialize;
use target_support::{TradeTestHarness, TradeTestTarget};
use tokio::sync::Mutex;

const ORDER_TEST_SYMBOL: &str = "SPY";
static ORDERS_API_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[tokio::test]
async fn orders_basic_lifecycle_live_paper() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_basic_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn orders_basic_lifecycle_mock() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_basic_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_limit_replace_live_paper() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_mleg_limit_replace_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_limit_replace_mock() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_mleg_limit_replace_scenario(&harness).await;
}

#[tokio::test]
async fn orders_cancel_all_live_paper() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_cancel_all_scenario(&harness).await;
}

#[tokio::test]
async fn orders_cancel_all_mock() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_cancel_all_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_live_paper() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_stop_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_limit_live_paper() {
    let _guard = orders_api_lock().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_stop_limit_scenario(&harness).await;
}

async fn orders_basic_lifecycle_scenario(harness: &TradeTestHarness) {
    let client = harness.trade_client();
    let listed = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(20),
            ..ListRequest::default()
        })
        .await
        .expect("orders list should remain readable");
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "list",
        &listed,
        "orders list sample should record",
    );

    if maybe_skip_live_market_session(harness, "basic order lifecycle").await {
        return;
    }

    let limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("non-marketable stock price should be discoverable from live market data");
    let client_order_id = unique_client_order_id(&format!(
        "phase12-{}-basic",
        target_slug(harness)
    ));

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("limit order create should succeed");
    assert_eq!(created.symbol, ORDER_TEST_SYMBOL);
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "created",
        &created,
        "created order sample should record",
    );

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("created order should remain readable");
    assert_eq!(fetched.id, created.id);

    let listed_open = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..ListRequest::default()
        })
        .await
        .expect("open orders list should expose the resting limit order");
    assert!(listed_open.iter().any(|order| order.id == created.id));

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    client
        .orders()
        .cancel(&created.id)
        .await
        .expect("limit order cancel should succeed");

    let canceled = wait_for_order_status(harness, &created.id, OrderStatus::Canceled)
        .await
        .expect("limit order should become canceled");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "canceled",
        &canceled,
        "canceled order sample should record",
    );
}

async fn orders_mleg_limit_replace_scenario(harness: &TradeTestHarness) {
    if maybe_skip_live_market_session(harness, "multi-leg replace lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let client = harness.trade_client();
    let spread = discover_mleg_call_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted multi-leg call spread should be discoverable from the live option chain");
    let client_order_id = unique_client_order_id(&format!(
        "phase20-{}-mleg",
        target_slug(harness)
    ));
    let replaced_client_order_id = format!("{client_order_id}-replaced");

    let created = client
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
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
            symbol: None,
        })
        .await
        .expect("multi-leg order create should succeed");
    assert_eq!(created.order_class, OrderClass::Mleg);
    assert!(created.legs.as_ref().is_some_and(|legs| legs.len() == spread.legs.len()));
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "mleg-created",
        &created,
        "multi-leg created order sample should record",
    );

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("created multi-leg order should remain readable");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("multi-leg client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let replacement = client
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
        .expect("multi-leg order replace should succeed");
    assert_ne!(replacement.id, created.id);
    assert_eq!(replacement.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(
        replacement.limit_price,
        Some(spread.more_conservative_limit_price)
    );
    assert_eq!(replacement.order_class, OrderClass::Mleg);
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "mleg-replaced",
        &replacement,
        "multi-leg replacement sample should record",
    );

    let replaced_source = wait_for_order_status(harness, &created.id, OrderStatus::Replaced)
        .await
        .expect("original multi-leg order should become replaced");
    assert_eq!(
        replaced_source.replaced_by.as_deref(),
        Some(replacement.id.as_str())
    );

    let replacement_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&replaced_client_order_id)
        .await
        .expect("replacement client_order_id lookup should succeed");
    assert_eq!(replacement_by_client_order_id.id, replacement.id);

    if harness.is_mock() {
        let created_legs = created
            .legs
            .as_ref()
            .expect("created mock mleg should keep nested legs");
        let replaced_legs = replacement
            .legs
            .as_ref()
            .expect("replacement mock mleg should keep nested legs");
        assert_eq!(replaced_legs.len(), created_legs.len());
        for (old_leg, new_leg) in created_legs.iter().zip(replaced_legs.iter()) {
            assert_ne!(new_leg.id, old_leg.id);
            assert_eq!(new_leg.replaces.as_deref(), Some(old_leg.id.as_str()));
        }
    }

    client
        .orders()
        .cancel(&replacement.id)
        .await
        .expect("replacement order cancel should succeed");

    let canceled = wait_for_order_status(harness, &replacement.id, OrderStatus::Canceled)
        .await
        .expect("replacement order should become canceled");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    if harness.is_mock() {
        assert!(
            canceled
                .legs
                .as_ref()
                .is_some_and(|legs| legs.iter().all(|leg| leg.status == OrderStatus::Canceled))
        );
    }
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "mleg-canceled",
        &canceled,
        "multi-leg canceled order sample should record",
    );
}

async fn orders_cancel_all_scenario(harness: &TradeTestHarness) {
    if maybe_skip_live_market_session(harness, "cancel_all lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let client = harness.trade_client();
    let stock_limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("non-marketable stock price should be discoverable for cancel_all");
    let spread = discover_mleg_call_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted multi-leg call spread should be discoverable for cancel_all");

    let stock_created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock_limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(unique_client_order_id(&format!(
                "phase20-{}-cancel-all-stock",
                target_slug(harness)
            ))),
            ..CreateRequest::default()
        })
        .await
        .expect("stock order create should succeed before cancel_all");
    let mleg_created = client
        .orders()
        .create(CreateRequest {
            symbol: None,
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
            client_order_id: Some(unique_client_order_id(&format!(
                "phase20-{}-cancel-all-mleg",
                target_slug(harness)
            ))),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("multi-leg order create should succeed before cancel_all");

    let canceled = client
        .orders()
        .cancel_all()
        .await
        .expect("cancel_all should succeed");
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "cancel-all",
        &canceled,
        "cancel_all sample should record",
    );
    assert!(canceled.iter().any(|result| result.id == stock_created.id));
    assert!(canceled.iter().any(|result| result.id == mleg_created.id));

    if harness.is_mock() {
        let mleg_cancel_body = canceled
            .iter()
            .find(|result| result.id == mleg_created.id)
            .and_then(|result| result.body.as_ref())
            .expect("mock cancel_all should return the canceled multi-leg order body");
        assert_eq!(mleg_cancel_body.status, OrderStatus::Canceled);
        assert!(
            mleg_cancel_body
                .legs
                .as_ref()
                .is_some_and(|legs| legs.iter().all(|leg| leg.status == OrderStatus::Canceled))
        );
    }

    for created_id in [&stock_created.id, &mleg_created.id] {
        let order = wait_for_order_status(harness, created_id, OrderStatus::Canceled)
            .await
            .expect("cancel_all should drive each created order to canceled");
        assert_eq!(order.status, OrderStatus::Canceled);
    }
}

async fn orders_stop_scenario(harness: &TradeTestHarness) {
    if maybe_skip_live_market_session(harness, "stop order lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("stop order price context should be discoverable");
    let client_order_id = unique_client_order_id("phase21-paper-stop");

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Stop),
            time_in_force: Some(TimeInForce::Day),
            limit_price: None,
            stop_price: Some(pricing.resting_buy_stop_price),
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("stop order create should succeed");
    assert_eq!(created.r#type, OrderType::Stop);
    assert_eq!(created.stop_price, Some(pricing.resting_buy_stop_price));

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("created stop order should remain readable");
    assert_eq!(fetched.id, created.id);

    let listed = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..ListRequest::default()
        })
        .await
        .expect("open orders list should expose the resting stop order");
    assert!(listed.iter().any(|order| order.id == created.id));

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("stop order client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    client
        .orders()
        .cancel(&created.id)
        .await
        .expect("stop order cancel should succeed");
    let canceled = wait_for_order_status(harness, &created.id, OrderStatus::Canceled)
        .await
        .expect("stop order should become canceled");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "paper-stop-canceled",
        &canceled,
        "stop order canceled sample should record",
    );
}

async fn orders_stop_limit_scenario(harness: &TradeTestHarness) {
    if maybe_skip_live_market_session(harness, "stop-limit order lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("stop-limit order price context should be discoverable");
    let client_order_id = unique_client_order_id("phase21-paper-stop-limit");

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::StopLimit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(pricing.resting_buy_stop_limit_price),
            stop_price: Some(pricing.resting_buy_stop_price),
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("stop-limit order create should succeed");
    assert_eq!(created.r#type, OrderType::StopLimit);
    assert_eq!(created.stop_price, Some(pricing.resting_buy_stop_price));
    assert_eq!(
        created.limit_price,
        Some(pricing.resting_buy_stop_limit_price)
    );

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("created stop-limit order should remain readable");
    assert_eq!(fetched.id, created.id);

    let listed = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(50),
            ..ListRequest::default()
        })
        .await
        .expect("open orders list should expose the resting stop-limit order");
    assert!(listed.iter().any(|order| order.id == created.id));

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("stop-limit client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    client
        .orders()
        .cancel(&created.id)
        .await
        .expect("stop-limit order cancel should succeed");
    let canceled = wait_for_order_status(harness, &created.id, OrderStatus::Canceled)
        .await
        .expect("stop-limit order should become canceled");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    maybe_record_live_json(
        harness,
        "alpaca-trade-orders",
        "paper-stop-limit-canceled",
        &canceled,
        "stop-limit order canceled sample should record",
    );
}

async fn maybe_skip_live_market_session(harness: &TradeTestHarness, scenario: &str) -> bool {
    let Some(paper_state) = harness.live_paper_session_state().await else {
        return false;
    };
    if can_submit_live_paper_orders(&paper_state) {
        return false;
    }

    eprintln!(
        "skipping {} {}: market session is unavailable",
        harness.label(),
        scenario
    );
    true
}

fn maybe_record_live_json<T>(
    harness: &TradeTestHarness,
    suite: &str,
    name: &str,
    payload: &T,
    context: &str,
) where
    T: Serialize,
{
    if let Some(recorder) = harness.recorder() {
        recorder
            .record_json(suite, name, payload)
            .expect(context);
    }
}

fn target_slug(harness: &TradeTestHarness) -> &'static str {
    if harness.is_mock() {
        "mock"
    } else {
        "paper"
    }
}

async fn wait_for_order_status(
    harness: &TradeTestHarness,
    order_id: &str,
    expected_status: OrderStatus,
) -> Result<alpaca_trade::orders::Order, Error> {
    for _attempt in 0..harness.poll_attempts() {
        let order = harness.trade_client().orders().get(order_id).await?;
        if order.status == expected_status {
            return Ok(order);
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    harness.trade_client().orders().get(order_id).await
}

async fn orders_api_lock() -> tokio::sync::MutexGuard<'static, ()> {
    ORDERS_API_MUTEX.get_or_init(|| Mutex::new(())).lock().await
}
