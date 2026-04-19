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
        CloseAllRequest, Position, option_qty_map, reconcile_signed_positions, structure_quantity,
    },
};
use rust_decimal::Decimal;
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
    assert_eq!(
        structure_quantity(
            template
                .iter()
                .map(|position| (position.symbol.as_str(), position.qty)),
            &mapped,
        ),
        Some(2)
    );

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
    reconcile_signed_positions(
        &mut reconciled,
        &mapped,
        |position| position.symbol.as_str(),
        |position, qty| position.qty = qty,
    );
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
async fn positions_close_all_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    positions_close_all_scenario(&harness).await;
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
