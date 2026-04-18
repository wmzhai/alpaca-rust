#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;
#[path = "support/trade_state.rs"]
mod trade_state_support;

use std::collections::BTreeSet;

use alpaca_trade::{
    orders::{CreateRequest, OrderSide, OrderType, TimeInForce},
    positions::{
        CloseAllRequest, ClosePositionRequest, Position, SignedPositionLike, option_qty_map,
        reconcile_signed_positions, structure_quantity,
    },
};
use rust_decimal::Decimal;
use serde::Serialize;
use target_support::{TradeTestHarness, TradeTestTarget};
use trade_state_support::{
    ensure_symbol_flat, wait_for_order_status, wait_for_position, wait_for_position_absent,
};

use order_support::unique_client_order_id;

const POSITION_TEST_SYMBOL: &str = "SPY";

#[derive(Clone, Debug, PartialEq)]
struct TemplatePosition {
    symbol: String,
    qty: i32,
}

impl SignedPositionLike for TemplatePosition {
    fn symbol(&self) -> &str {
        &self.symbol
    }

    fn signed_qty(&self) -> i32 {
        self.qty
    }

    fn set_signed_qty(&mut self, qty: i32) {
        self.qty = qty;
    }
}

#[test]
fn positions_convenience_maps_resolves_and_reconciles_option_shapes() {
    let live_positions = vec![
        Position {
            symbol: "SPY260417C00550000".to_string(),
            qty: Decimal::from(2),
            ..Position::default()
        },
        Position {
            symbol: "SPY260417P00530000".to_string(),
            qty: Decimal::from(-2),
            ..Position::default()
        },
        Position {
            symbol: "SPY".to_string(),
            qty: Decimal::from(10),
            ..Position::default()
        },
    ];
    let mapped = option_qty_map(&live_positions);
    assert_eq!(mapped.get("SPY260417C00550000"), Some(&2));
    assert_eq!(mapped.get("SPY260417P00530000"), Some(&-2));
    assert!(!mapped.contains_key("SPY"));

    let template = vec![
        TemplatePosition {
            symbol: "SPY260417C00550000".to_string(),
            qty: 1,
        },
        TemplatePosition {
            symbol: "SPY260417P00530000".to_string(),
            qty: -1,
        },
    ];
    assert_eq!(structure_quantity(&template, &mapped), Some(2));

    let mut reconciled = vec![
        TemplatePosition {
            symbol: "SPY260417C00550000".to_string(),
            qty: 1,
        },
        TemplatePosition {
            symbol: "SPY260417P00530000".to_string(),
            qty: -1,
        },
        TemplatePosition {
            symbol: "SPY260417C00999000".to_string(),
            qty: 1,
        },
    ];
    reconcile_signed_positions(&mut reconciled, &mapped);
    assert_eq!(
        reconciled,
        vec![
            TemplatePosition {
                symbol: "SPY260417C00550000".to_string(),
                qty: 2,
            },
            TemplatePosition {
                symbol: "SPY260417P00530000".to_string(),
                qty: -2,
            },
        ]
    );
}

#[tokio::test]
async fn positions_equity_lifecycle_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
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

#[tokio::test]
async fn positions_close_all_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    positions_close_all_scenario(&harness).await;
}

async fn positions_equity_lifecycle_scenario(harness: &TradeTestHarness) {
    if harness
        .should_skip_live_market_session("positions equity lifecycle")
        .await
    {
        return;
    }

    ensure_symbol_flat(harness, POSITION_TEST_SYMBOL).await;
    let cash_before_open = account_cash(harness).await;

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
    let opened = wait_for_order_status(
        harness,
        &opened.id,
        alpaca_trade::orders::OrderStatus::Filled,
    )
    .await;
    let cash_after_open = account_cash(harness).await;
    assert!(cash_after_open < cash_before_open);
    assert_cash_delta_equals_fill_value(
        cash_before_open,
        cash_after_open,
        opened
            .filled_avg_price
            .expect("filled open order should expose filled_avg_price"),
        opened.filled_qty,
    );

    let position = wait_for_position(harness, POSITION_TEST_SYMBOL).await;
    assert_eq!(position.qty, opened.filled_qty);
    assert_eq!(position.asset_class, "us_equity");
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
    assert_eq!(by_symbol.qty, opened.filled_qty);

    let cash_before_close = account_cash(harness).await;
    let close = harness
        .trade_client()
        .positions()
        .close(POSITION_TEST_SYMBOL, ClosePositionRequest::default())
        .await
        .expect("close position should submit");
    let closed = wait_for_order_status(
        harness,
        &close.id,
        alpaca_trade::orders::OrderStatus::Filled,
    )
    .await;
    let cash_after_close = account_cash(harness).await;
    assert!(cash_after_close > cash_before_close);
    assert_cash_delta_equals_fill_value(
        cash_before_close,
        cash_after_close,
        closed
            .filled_avg_price
            .expect("filled close order should expose filled_avg_price"),
        closed.filled_qty,
    );
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

async fn positions_close_all_scenario(harness: &TradeTestHarness) {
    if harness
        .should_skip_live_market_session("positions close_all lifecycle")
        .await
    {
        return;
    }

    let created_symbols = ["QQQ", POSITION_TEST_SYMBOL]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for symbol in &created_symbols {
        ensure_symbol_flat(harness, symbol).await;
    }

    let result: Result<(), alpaca_trade::Error> = async {
        for symbol in &created_symbols {
            let opened = open_stock_position(
                harness,
                symbol,
                &unique_client_order_id(&format!("phase21-{}-close-all-{symbol}", harness.slug())),
            )
            .await;
            let position = wait_for_position(harness, symbol).await;
            assert_eq!(position.qty, opened.filled_qty);
        }

        let close_results = harness
            .trade_client()
            .positions()
            .close_all(CloseAllRequest::default())
            .await
            .expect("close_all should submit");
        let closed_symbols = close_results
            .iter()
            .map(|result| result.symbol.clone())
            .collect::<BTreeSet<_>>();
        assert!(created_symbols.is_subset(&closed_symbols));

        for close_result in close_results
            .iter()
            .filter(|result| created_symbols.contains(&result.symbol))
        {
            let close_body = close_result
                .body
                .as_ref()
                .expect("close_all should return a typed order body for created symbols");
            let closed = wait_for_order_status(
                harness,
                &close_body.id,
                alpaca_trade::orders::OrderStatus::Filled,
            )
            .await;
            assert_eq!(closed.status, alpaca_trade::orders::OrderStatus::Filled);
        }

        for symbol in &created_symbols {
            wait_for_position_absent(harness, symbol).await;
        }

        Ok(())
    }
    .await;

    for symbol in &created_symbols {
        ensure_symbol_flat(harness, symbol).await;
    }

    result.expect("positions close_all should close the test-created positions");
}

async fn open_stock_position(
    harness: &TradeTestHarness,
    symbol: &str,
    client_order_id: &str,
) -> alpaca_trade::orders::Order {
    let created = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(client_order_id.to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("open stock position order should submit");
    wait_for_order_status(
        harness,
        &created.id,
        alpaca_trade::orders::OrderStatus::Filled,
    )
    .await
}

async fn account_cash(harness: &TradeTestHarness) -> Decimal {
    harness
        .trade_client()
        .account()
        .get()
        .await
        .expect("account should remain readable")
        .cash
        .expect("account cash should be present")
}

fn assert_cash_delta_equals_fill_value(
    before: Decimal,
    after: Decimal,
    fill_price: Decimal,
    fill_qty: Decimal,
) {
    assert_eq!(
        (after - before).abs(),
        fill_price * fill_qty,
        "cash delta should equal fill_price * fill_qty",
    );
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
        recorder.record_json(suite, name, payload).expect(context);
    }
}
