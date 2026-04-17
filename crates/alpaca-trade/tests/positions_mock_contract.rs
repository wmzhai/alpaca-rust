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
    orders::{CreateRequest, OrderClass, OrderSide, OrderStatus, OrderType, TimeInForce},
    positions::CloseAllRequest,
};
use order_support::{
    clear_option_universe_cache, discover_mleg_call_broken_wing_butterfly,
    discover_mleg_call_spread, unique_client_order_id,
};
use rust_decimal::Decimal;
use target_support::TradeTestTarget;
use trade_state_support::{wait_for_order_status, wait_for_position, wait_for_position_absent};

const SYMBOL_A: &str = "SPY";
const SYMBOL_B: &str = "QQQ";

#[tokio::test]
async fn positions_mock_close_all_closes_remaining_open_positions() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    let opened_spy = open_mock_position(&harness, SYMBOL_A, "phase13-mock-spy-open").await;
    let opened_qqq = open_mock_position(&harness, SYMBOL_B, "phase13-mock-qqq-open").await;

    let listed = harness
        .trade_client()
        .positions()
        .list()
        .await
        .expect("mock positions list should succeed");
    let listed_symbols = listed
        .iter()
        .map(|position| position.symbol.clone())
        .collect::<BTreeSet<_>>();
    assert!(listed_symbols.contains(SYMBOL_A));
    assert!(listed_symbols.contains(SYMBOL_B));

    let close_results = harness
        .trade_client()
        .positions()
        .close_all(CloseAllRequest::default())
        .await
        .expect("mock close_all should succeed");
    let close_all_symbols = close_results
        .iter()
        .map(|result| result.symbol.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        close_all_symbols,
        BTreeSet::from([SYMBOL_A.to_owned(), SYMBOL_B.to_owned()])
    );

    for result in &close_results {
        let close_order_id = &result
            .body
            .as_ref()
            .expect("mock close_all body should be present")
            .id;
        let closed = harness
            .trade_client()
            .orders()
            .get(close_order_id)
            .await
            .expect("mock close_all order should be readable");
        assert_eq!(closed.status, OrderStatus::Filled);
    }
    wait_for_position_absent(&harness, SYMBOL_A).await;
    wait_for_position_absent(&harness, SYMBOL_B).await;

    assert_ne!(opened_spy.asset_id, opened_qqq.asset_id);
}

#[tokio::test]
async fn positions_mock_projects_filled_mleg_legs_without_parent_combo_position() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let spread = discover_mleg_call_spread(harness.data_client(), SYMBOL_A)
        .await
        .expect("dynamic call spread should be discoverable");

    let created = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            limit_price: None,
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(unique_client_order_id("phase20-mock-positions-mleg")),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("mock market multi-leg order should succeed");
    let created = wait_for_order_status(&harness, &created.id, OrderStatus::Filled).await;
    assert_eq!(created.status, OrderStatus::Filled);

    let positions = harness
        .trade_client()
        .positions()
        .list()
        .await
        .expect("mock positions list should succeed after mleg fill");
    assert_eq!(positions.len(), spread.legs.len());
    assert!(positions.iter().all(|position| !position.symbol.is_empty()));
    assert!(
        positions
            .iter()
            .all(|position| position.symbol != created.symbol)
    );
    for leg in &spread.legs {
        let position = positions
            .iter()
            .find(|position| position.symbol == leg.symbol)
            .expect("each spread leg should project into a position");
        assert_eq!(position.asset_class, "us_option");
        assert_eq!(
            position.qty,
            match leg.side {
                Some(OrderSide::Buy) => Decimal::ONE,
                Some(OrderSide::Sell) => -Decimal::ONE,
                _ => panic!("spread leg should have a side"),
            }
        );
        assert_eq!(
            position.side,
            match leg.side {
                Some(OrderSide::Buy) => "long",
                Some(OrderSide::Sell) => "short",
                _ => panic!("spread leg should have a side"),
            }
        );
    }
}

#[tokio::test]
async fn positions_mock_projects_bwb_legs_scaled_by_parent_qty() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let bwb = discover_mleg_call_broken_wing_butterfly(harness.data_client(), SYMBOL_A)
        .await
        .expect("dynamic 1:2:1 call structure should be discoverable");

    let created = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(Decimal::new(2, 0)),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            limit_price: None,
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(unique_client_order_id("phase20-mock-positions-bwb")),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(bwb.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("mock market 1:2:1 multi-leg order should succeed");
    let created = wait_for_order_status(&harness, &created.id, OrderStatus::Filled).await;
    assert_eq!(created.status, OrderStatus::Filled);

    let positions = harness
        .trade_client()
        .positions()
        .list()
        .await
        .expect("mock positions list should succeed after 1:2:1 fill");
    assert_eq!(positions.len(), bwb.legs.len());

    for leg in &bwb.legs {
        let position = positions
            .iter()
            .find(|position| position.symbol == leg.symbol)
            .expect("each 1:2:1 leg should project into a position");
        assert_eq!(position.asset_class, "us_option");
        assert_eq!(
            position.qty,
            match leg.side {
                Some(OrderSide::Buy) => Decimal::from(leg.ratio_qty * 2),
                Some(OrderSide::Sell) => -Decimal::from(leg.ratio_qty * 2),
                _ => panic!("1:2:1 leg should have a side"),
            }
        );
        assert_eq!(
            position.side,
            match leg.side {
                Some(OrderSide::Buy) => "long",
                Some(OrderSide::Sell) => "short",
                _ => panic!("1:2:1 leg should have a side"),
            }
        );
    }
}

async fn open_mock_position(
    harness: &target_support::TradeTestHarness,
    symbol: &str,
    prefix: &str,
) -> alpaca_trade::positions::Position {
    let order = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some(unique_client_order_id(prefix)),
            ..CreateRequest::default()
        })
        .await
        .expect("mock open order should succeed");
    let order = wait_for_order_status(harness, &order.id, OrderStatus::Filled).await;
    assert_eq!(order.status, OrderStatus::Filled);

    wait_for_position(harness, symbol).await
}
