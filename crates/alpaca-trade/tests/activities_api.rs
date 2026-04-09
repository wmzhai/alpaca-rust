#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;
#[path = "support/trade_state.rs"]
mod trade_state_support;

use alpaca_trade::{
    activities::{ListByTypeRequest, ListRequest},
    orders::{CreateRequest, OrderSide, OrderType, SortDirection, TimeInForce},
    positions::ClosePositionRequest,
};
use live_support::trading_day_from_timestamp;
use order_support::unique_client_order_id;
use rust_decimal::Decimal;
use serde::Serialize;
use target_support::{TradeTestHarness, TradeTestTarget};
use trade_state_support::{ensure_symbol_flat, wait_for_order_status};

const ACTIVITY_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn activities_fill_lifecycle_live_paper() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    activities_fill_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn activities_fill_lifecycle_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    activities_fill_lifecycle_scenario(&harness).await;
}

async fn activities_fill_lifecycle_scenario(harness: &TradeTestHarness) {
    if harness
        .should_skip_live_market_session("activities fill lifecycle")
        .await
    {
        return;
    }

    ensure_symbol_flat(harness, ACTIVITY_TEST_SYMBOL).await;
    let trading_day = harness
        .live_paper_session_state()
        .await
        .map(|state| {
            trading_day_from_timestamp(&state.clock.timestamp)
                .expect("paper clock timestamp should contain a trading day")
        });

    let opened = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(ACTIVITY_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(unique_client_order_id(&format!(
                "phase14-{}-activity-open",
                harness.slug()
            ))),
            ..CreateRequest::default()
        })
        .await
        .expect("open order should submit");
    let opened =
        wait_for_order_status(harness, &opened.id, alpaca_trade::orders::OrderStatus::Filled).await;

    let close = harness
        .trade_client()
        .positions()
        .close(ACTIVITY_TEST_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit");
    let closed =
        wait_for_order_status(harness, &close.id, alpaca_trade::orders::OrderStatus::Filled).await;

    let fills = wait_for_fill_activities(
        harness,
        &[opened.id.as_str(), closed.id.as_str()],
        trading_day.as_deref(),
    )
    .await;
    maybe_record_live_json(
        harness,
        "alpaca-trade-activities",
        "fills",
        &fills,
        "fill activities sample should record",
    );
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
    assert!(fills.iter().all(|activity| activity.activity_type == "FILL"));

    let fills_by_type = harness
        .trade_client()
        .activities()
        .list_by_type(
            "FILL",
            ListByTypeRequest {
                date: trading_day.clone(),
                direction: Some(SortDirection::Desc),
                page_size: Some(100),
                ..ListByTypeRequest::default()
            },
        )
        .await
        .expect("typed activities endpoint should succeed");
    assert!(
        fills_by_type
            .iter()
            .all(|activity| activity.activity_type == "FILL")
    );
    assert!(
        fills_by_type
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&opened.id))
    );
    assert!(
        fills_by_type
            .iter()
            .any(|activity| activity.order_id.as_deref() == Some(&closed.id))
    );
}

async fn wait_for_fill_activities(
    harness: &TradeTestHarness,
    order_ids: &[&str],
    trading_day: Option<&str>,
) -> Vec<alpaca_trade::activities::Activity> {
    for _attempt in 0..(harness.poll_attempts() * 3) {
        let fills = harness
            .trade_client()
            .activities()
            .list(ListRequest {
                activity_types: Some(vec!["FILL".to_owned()]),
                date: trading_day.map(ToOwned::to_owned),
                direction: Some(SortDirection::Desc),
                page_size: Some(100),
                ..ListRequest::default()
            })
            .await
            .expect("fill activities should remain readable");
        if order_ids.iter().all(|order_id| {
            fills
                .iter()
                .any(|activity| activity.order_id.as_deref() == Some(*order_id))
        }) {
            return fills;
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    harness
        .trade_client()
        .activities()
        .list(ListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            date: trading_day.map(ToOwned::to_owned),
            direction: Some(SortDirection::Desc),
            page_size: Some(100),
            ..ListRequest::default()
        })
        .await
        .expect("fill activities should remain readable")
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
