#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;
#[path = "support/trade_state.rs"]
mod trade_state_support;

use alpaca_trade::{
    activities::ListRequest as ActivitiesListRequest,
    orders::{
        CreateRequest, ListRequest as OrdersListRequest, OrderSide, QueryOrderStatus,
        SortDirection, TimeInForce,
    },
    positions::ClosePositionRequest,
};
use live_support::trading_day_from_timestamp;
use order_support::unique_client_order_id;
use rust_decimal::Decimal;
use serde_json::json;
use target_support::{TradeTestHarness, TradeTestTarget};
use trade_state_support::{
    ensure_symbol_flat, wait_for_order_status, wait_for_position, wait_for_position_absent,
};

const MAINLINE_SYMBOL: &str = "SPY";

#[tokio::test]
async fn trade_mainline_lifecycle_live_paper() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    trade_mainline_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn trade_mainline_lifecycle_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    trade_mainline_lifecycle_scenario(&harness).await;
}

async fn trade_mainline_lifecycle_scenario(harness: &TradeTestHarness) {
    if harness
        .should_skip_live_market_session("mainline lifecycle")
        .await
    {
        return;
    }

    ensure_symbol_flat(harness, MAINLINE_SYMBOL).await;
    let trading_day = harness
        .live_paper_session_state()
        .await
        .map(|state| {
            trading_day_from_timestamp(&state.clock.timestamp)
                .expect("paper clock timestamp should contain a trading day")
        });

    let account_before = harness
        .trade_client()
        .account()
        .get()
        .await
        .expect("account should be readable before the lifecycle starts");

    let opened = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(MAINLINE_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(alpaca_trade::orders::OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(unique_client_order_id(&format!(
                "phase15-{}-mainline-open",
                harness.slug()
            ))),
            ..CreateRequest::default()
        })
        .await
        .expect("open order should submit");
    let opened =
        wait_for_order_status(harness, &opened.id, alpaca_trade::orders::OrderStatus::Filled).await;

    let opened_position = wait_for_position(harness, MAINLINE_SYMBOL).await;
    assert_eq!(opened_position.symbol, MAINLINE_SYMBOL);

    let fills_after_open = wait_for_fill_activity(harness, &opened.id, trading_day.as_deref()).await;
    assert!(
        fills_after_open
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&opened.id))
    );

    let close = harness
        .trade_client()
        .positions()
        .close(MAINLINE_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit");
    let closed =
        wait_for_order_status(harness, &close.id, alpaca_trade::orders::OrderStatus::Filled).await;
    wait_for_position_absent(harness, MAINLINE_SYMBOL).await;

    let fills_after_close =
        wait_for_fill_activity(harness, &closed.id, trading_day.as_deref()).await;
    assert!(
        fills_after_close
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&closed.id))
    );

    let orders = harness
        .trade_client()
        .orders()
        .list(OrdersListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(50),
            ..OrdersListRequest::default()
        })
        .await
        .expect("orders list should expose the full lifecycle");
    assert!(orders.iter().any(|order| order.id == opened.id));
    assert!(orders.iter().any(|order| order.id == closed.id));

    let account_after = harness
        .trade_client()
        .account()
        .get()
        .await
        .expect("account should remain readable after the lifecycle");
    assert_eq!(account_before.id, account_after.id);
    assert!(account_before.cash.is_some());
    assert!(account_after.cash.is_some());

    if let Some(recorder) = harness.recorder() {
        recorder
            .record_json(
                "alpaca-trade-mainline",
                "lifecycle",
                &json!({
                    "account_before": account_before,
                    "open_order": opened,
                    "open_position": opened_position,
                    "close_order": closed,
                    "fills_after_close": fills_after_close,
                    "account_after": account_after,
                }),
            )
            .expect("mainline lifecycle sample should record");
    }
}

async fn wait_for_fill_activity(
    harness: &TradeTestHarness,
    order_id: &str,
    trading_day: Option<&str>,
) -> Vec<alpaca_trade::activities::Activity> {
    for _attempt in 0..(harness.poll_attempts() * 3) {
        let fills = harness
            .trade_client()
            .activities()
            .list(ActivitiesListRequest {
                activity_types: Some(vec!["FILL".to_owned()]),
                date: trading_day.map(ToOwned::to_owned),
                direction: Some(SortDirection::Desc),
                page_size: Some(100),
                ..ActivitiesListRequest::default()
            })
            .await
            .expect("fill activities should remain readable");
        if fills
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(order_id))
        {
            return fills;
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    harness
        .trade_client()
        .activities()
        .list(ActivitiesListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            date: trading_day.map(ToOwned::to_owned),
            direction: Some(SortDirection::Desc),
            page_size: Some(100),
            ..ActivitiesListRequest::default()
        })
        .await
        .expect("fill activities should remain readable")
}
