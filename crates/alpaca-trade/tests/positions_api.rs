#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;
#[path = "support/trade_state.rs"]
mod trade_state_support;

use alpaca_trade::{
    orders::{CreateRequest, OrderSide, OrderType, TimeInForce},
    positions::ClosePositionRequest,
};
use rust_decimal::Decimal;
use serde::Serialize;
use target_support::{TradeTestHarness, TradeTestTarget};
use trade_state_support::{
    ensure_symbol_flat, wait_for_order_status, wait_for_position, wait_for_position_absent,
};

use order_support::unique_client_order_id;

const POSITION_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn positions_equity_lifecycle_live_paper() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    positions_equity_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn positions_equity_lifecycle_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    positions_equity_lifecycle_scenario(&harness).await;
}

async fn positions_equity_lifecycle_scenario(harness: &TradeTestHarness) {
    if harness
        .should_skip_live_market_session("positions equity lifecycle")
        .await
    {
        return;
    }

    ensure_symbol_flat(harness, POSITION_TEST_SYMBOL).await;

    let opened = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(POSITION_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(unique_client_order_id(&format!(
                "phase13-{}-open",
                harness.slug()
            ))),
            ..CreateRequest::default()
        })
        .await
        .expect("open order should submit");
    let opened = wait_for_order_status(harness, &opened.id, alpaca_trade::orders::OrderStatus::Filled).await;

    let position = wait_for_position(harness, POSITION_TEST_SYMBOL).await;
    maybe_record_live_json(
        harness,
        "alpaca-trade-positions",
        "open-position",
        &position,
        "opened position sample should record",
    );

    let listed = harness
        .trade_client()
        .positions()
        .list()
        .await
        .expect("positions list should succeed");
    assert!(listed.iter().any(|candidate| {
        candidate.symbol == POSITION_TEST_SYMBOL && candidate.asset_id == position.asset_id
    }));

    let by_symbol = harness
        .trade_client()
        .positions()
        .get(POSITION_TEST_SYMBOL)
        .await
        .expect("position get by symbol should succeed");
    let by_asset_id = harness
        .trade_client()
        .positions()
        .get(&position.asset_id)
        .await
        .expect("position get by asset id should succeed");
    assert_eq!(by_symbol.asset_id, by_asset_id.asset_id);

    let close = harness
        .trade_client()
        .positions()
        .close(POSITION_TEST_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit");
    let closed = wait_for_order_status(harness, &close.id, alpaca_trade::orders::OrderStatus::Filled).await;
    maybe_record_live_json(
        harness,
        "alpaca-trade-positions",
        "close-order",
        &closed,
        "closed order sample should record",
    );
    wait_for_position_absent(harness, POSITION_TEST_SYMBOL).await;

    assert_eq!(opened.asset_id, closed.asset_id);
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
