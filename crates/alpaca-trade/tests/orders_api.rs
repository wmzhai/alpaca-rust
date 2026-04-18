#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;
#[path = "support/trade_state.rs"]
mod trade_state_support;

use alpaca_data::{
    Client as DataClient,
    options::{OptionsFeed, SnapshotsRequest},
};
use alpaca_mock::{
    InjectedHttpFault, LiveMarketDataBridge, MockServerState, TestServer,
    spawn_test_server_with_state,
};
use alpaca_trade::{
    Client as TradeClient, Error,
    orders::{
        CloseOptionLeg, CloseOptionLegsStatus, CreateRequest, ListRequest, OptionLegRequest,
        OptionQuote, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
        QueryOrderStatus, ReplaceRequest, ReplaceResolution, StopLoss, SubmitOrderStyle,
        TakeProfit, TimeInForce, WaitFor,
    },
};
use live_support::can_submit_live_paper_orders;
use order_support::{
    StockOrderPriceContext, clear_option_universe_cache, discover_distinct_mleg_call_spread_pair,
    discover_mleg_call_broken_wing_butterfly, discover_mleg_call_spread, discover_mleg_put_spread,
    discover_single_leg_call, non_marketable_buy_limit_price, stock_order_price_context,
    unique_client_order_id,
};
use rust_decimal::Decimal;
use serde::Serialize;
use target_support::{TradeTestHarness, TradeTestTarget};
use trade_state_support::{ensure_symbol_flat, wait_for_position, wait_for_position_absent};

const ORDER_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn orders_basic_lifecycle_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_basic_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn orders_basic_lifecycle_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_basic_lifecycle_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_limit_replace_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_mleg_limit_replace_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_limit_replace_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_mleg_limit_replace_scenario(&harness).await;
}

#[tokio::test]
async fn orders_cancel_all_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_cancel_all_scenario(&harness).await;
}

#[tokio::test]
async fn orders_cancel_all_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_cancel_all_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_stop_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_stop_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_limit_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_stop_limit_scenario(&harness).await;
}

#[tokio::test]
async fn orders_stop_limit_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_stop_limit_scenario(&harness).await;
}

#[tokio::test]
async fn orders_trailing_stop_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_trailing_stop_scenario(&harness).await;
}

#[tokio::test]
async fn orders_trailing_stop_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_trailing_stop_scenario(&harness).await;
}

#[tokio::test]
async fn orders_fractional_qty_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_fractional_qty_scenario(&harness).await;
}

#[tokio::test]
async fn orders_fractional_qty_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_fractional_qty_scenario(&harness).await;
}

#[tokio::test]
async fn orders_notional_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_notional_scenario(&harness).await;
}

#[tokio::test]
async fn orders_notional_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_notional_scenario(&harness).await;
}

#[tokio::test]
async fn orders_extended_hours_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_extended_hours_scenario(&harness).await;
}

#[tokio::test]
async fn orders_extended_hours_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_extended_hours_scenario(&harness).await;
}

#[tokio::test]
async fn orders_bracket_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_bracket_scenario(&harness).await;
}

#[tokio::test]
async fn orders_bracket_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_bracket_scenario(&harness).await;
}

#[tokio::test]
async fn orders_oto_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_oto_scenario(&harness).await;
}

#[tokio::test]
async fn orders_oto_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_oto_scenario(&harness).await;
}

#[tokio::test]
async fn orders_oco_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_oco_scenario(&harness).await;
}

#[tokio::test]
async fn orders_oco_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_oco_scenario(&harness).await;
}

#[tokio::test]
async fn orders_option_limit_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_option_limit_scenario(&harness).await;
}

#[tokio::test]
async fn orders_option_limit_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_option_limit_scenario(&harness).await;
}

#[tokio::test]
async fn orders_option_market_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_option_market_scenario(&harness).await;
}

#[tokio::test]
async fn orders_option_market_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_option_market_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_market_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_mleg_market_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_market_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_mleg_market_scenario(&harness).await;
}

#[tokio::test]
async fn orders_create_resolved_market_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    ensure_symbol_flat(&harness, ORDER_TEST_SYMBOL).await;

    let resolved = harness
        .trade_client()
        .orders()
        .create_resolved(
            CreateRequest::simple(
                ORDER_TEST_SYMBOL,
                1,
                OrderSide::Buy,
                SubmitOrderStyle::Market,
                None,
                None,
            )
            .expect("simple market request should build"),
            WaitFor::Filled,
        )
        .await
        .expect("create_resolved should submit and wait for fill");

    assert_eq!(resolved.order.status, OrderStatus::Filled);
    assert!(resolved.order.filled_avg_price.is_some());

    let closed = harness
        .trade_client()
        .positions()
        .close(ORDER_TEST_SYMBOL, Default::default())
        .await
        .expect("close position should submit");
    let _ = wait_for_order_status(&harness, &closed.id, OrderStatus::Filled).await;
    wait_for_position_absent(&harness, ORDER_TEST_SYMBOL).await;
}

#[tokio::test]
async fn orders_create_resolved_limit_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    ensure_symbol_flat(&harness, ORDER_TEST_SYMBOL).await;

    let limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("non marketable price should build");
    let resolved = harness
        .trade_client()
        .orders()
        .create_resolved(
            CreateRequest::simple(
                ORDER_TEST_SYMBOL,
                1,
                OrderSide::Buy,
                SubmitOrderStyle::Limit { limit_price },
                None,
                None,
            )
            .expect("simple limit request should build"),
            WaitFor::Stable,
        )
        .await
        .expect("create_resolved should submit and wait for stable status");

    assert!(
        matches!(
            resolved.order.status,
            OrderStatus::Accepted | OrderStatus::New
        ),
        "expected accepted/new after waiting for stable status, got {:?}",
        resolved.order.status
    );

    let canceled = harness
        .trade_client()
        .orders()
        .cancel_resolved(&resolved.order.id)
        .await
        .expect("cancel_resolved should cancel resting limit order");
    assert!(matches!(
        canceled.order.status,
        OrderStatus::Canceled | OrderStatus::Filled
    ));
}

#[tokio::test]
async fn orders_close_option_legs_all_liquid_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let spread = discover_mleg_call_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("mock call spread should be discoverable");
    let opened = submit_mleg_market_order(
        &harness,
        spread.legs.clone(),
        Decimal::from(2),
        OrderSide::Buy,
        &unique_client_order_id("phase22-mock-close-option-legs-all-liquid-open"),
    )
    .await
    .expect("mock multi-leg open should fill");

    let closing_legs = build_close_option_legs(&harness, &opened).await;
    let result = harness
        .trade_client()
        .orders()
        .close_option_legs(2, closing_legs)
        .await
        .expect("all-liquid close_option_legs should succeed");

    assert_eq!(result.status, CloseOptionLegsStatus::Filled);
    assert!(
        result
            .order
            .as_ref()
            .is_some_and(|order| order.status == OrderStatus::Filled)
    );
    assert!(result.cashflow != Decimal::ZERO);
    assert!(
        result
            .legs
            .iter()
            .all(|leg| leg.filled_avg_price > Decimal::ZERO)
    );

    for leg in opened
        .legs
        .as_ref()
        .expect("filled multi-leg order should expose nested legs")
    {
        wait_for_position_absent(&harness, &leg.symbol).await;
    }
}

#[tokio::test]
async fn orders_close_option_legs_scales_single_leg_ratio_qty_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let bwb = discover_mleg_call_broken_wing_butterfly(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("mock broken wing butterfly should be discoverable");
    let opened = submit_mleg_market_order(
        &harness,
        bwb.legs.clone(),
        Decimal::ONE,
        OrderSide::Buy,
        &unique_client_order_id("phase22-mock-close-option-legs-ratio-open"),
    )
    .await
    .expect("mock broken wing butterfly open should fill");

    let mut closing_legs = build_close_option_legs(&harness, &opened).await;
    let liquid_index = closing_legs
        .iter()
        .position(|leg| leg.ratio_qty == 2)
        .expect("broken wing butterfly should include a 2x leg");
    for (index, leg) in closing_legs.iter_mut().enumerate() {
        if index != liquid_index {
            leg.quote = None;
        }
    }

    let opened_legs = opened
        .legs
        .as_ref()
        .expect("filled multi-leg order should expose nested legs");
    let liquid_symbol = opened_legs[liquid_index].symbol.clone();
    let remaining_symbols = opened_legs
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != liquid_index)
        .map(|(_, leg)| leg.symbol.clone())
        .collect::<Vec<_>>();

    let result = harness
        .trade_client()
        .orders()
        .close_option_legs(1, closing_legs)
        .await
        .expect("single-liquid close_option_legs should succeed");

    assert_eq!(result.status, CloseOptionLegsStatus::Filled);
    assert!(
        result
            .order
            .as_ref()
            .is_some_and(|order| order.status == OrderStatus::Filled)
    );
    assert_eq!(result.legs[liquid_index].ratio_qty, 2);
    assert!(result.legs[liquid_index].filled_avg_price > Decimal::ZERO);
    assert!(
        result
            .legs
            .iter()
            .enumerate()
            .all(|(index, leg)| if index == liquid_index {
                leg.filled_avg_price > Decimal::ZERO
            } else {
                leg.filled_avg_price == Decimal::ZERO
            })
    );

    wait_for_position_absent(&harness, &liquid_symbol).await;
    for symbol in remaining_symbols {
        let position = wait_for_position(&harness, &symbol).await;
        assert!(position.qty != Decimal::ZERO);
    }
}

#[tokio::test]
async fn orders_close_option_legs_returns_zero_when_all_illiquid_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let spread = discover_mleg_put_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("mock put spread should be discoverable");
    let opened = submit_mleg_market_order(
        &harness,
        spread.legs.clone(),
        Decimal::ONE,
        OrderSide::Buy,
        &unique_client_order_id("phase22-mock-close-option-legs-all-illiquid-open"),
    )
    .await
    .expect("mock put spread open should fill");

    let mut closing_legs = build_close_option_legs(&harness, &opened).await;
    for leg in &mut closing_legs {
        leg.quote = None;
    }

    let tracked_symbols = opened
        .legs
        .as_ref()
        .expect("filled multi-leg order should expose nested legs")
        .iter()
        .map(|leg| leg.symbol.clone())
        .collect::<Vec<_>>();

    let result = harness
        .trade_client()
        .orders()
        .close_option_legs(1, closing_legs)
        .await
        .expect("all-illiquid close_option_legs should still succeed");

    assert_eq!(result.status, CloseOptionLegsStatus::Skipped);
    assert!(result.order.is_none());
    assert_eq!(result.cashflow, Decimal::ZERO);
    assert!(
        result
            .legs
            .iter()
            .all(|leg| leg.filled_avg_price == Decimal::ZERO)
    );

    for symbol in tracked_symbols {
        let position = wait_for_position(&harness, &symbol).await;
        assert!(position.qty != Decimal::ZERO);
    }
}

#[tokio::test]
async fn orders_cancel_resolved_recovers_after_request_error_mock() {
    let Some((_server, state, client)) = build_recovery_test_client().await else {
        return;
    };

    let created = create_non_marketable_spy_limit_order(&client).await;
    state
        .cancel_order("mock-key", &created.id)
        .expect("pre-cancel should succeed");
    state.set_http_fault(
        InjectedHttpFault::new(503, "injected cancel fault".to_owned())
            .expect("fault should build"),
    );

    let resolved = client
        .orders()
        .cancel_resolved(&created.id)
        .await
        .expect("cancel_resolved should recover after request error");

    assert!(resolved.recovered_after_request_error);
    assert_eq!(resolved.order.id, created.id);
    assert_eq!(resolved.order.status, OrderStatus::Canceled);
}

#[tokio::test]
async fn orders_replace_resolved_recovers_after_request_error_mock() {
    let Some((_server, state, client)) = build_recovery_test_client().await else {
        return;
    };

    let created = create_non_marketable_spy_limit_order(&client).await;
    let replacement = state
        .replace_order(
            "mock-key",
            &created.id,
            alpaca_mock::state::ReplaceOrderInput {
                limit_price: Some(Decimal::from(2)),
                ..Default::default()
            },
        )
        .await
        .expect("pre-replace should succeed");
    state.set_http_fault(
        InjectedHttpFault::new(503, "injected replace fault".to_owned())
            .expect("fault should build"),
    );

    let resolution = client
        .orders()
        .replace_resolved(
            &created.id,
            ReplaceRequest {
                limit_price: Some(Decimal::from(3)),
                ..ReplaceRequest::default()
            },
        )
        .await
        .expect("replace_resolved should recover after request error");

    match resolution {
        ReplaceResolution::NewOrder(resolved) => {
            assert!(resolved.recovered_after_request_error);
            assert_eq!(resolved.order.id, replacement.id);
            assert_eq!(resolved.order.status, OrderStatus::New);
            assert_eq!(
                resolved.order.replaces.as_deref(),
                Some(created.id.as_str())
            );
        }
        ReplaceResolution::OriginalOrderTerminal(resolved) => {
            panic!(
                "expected replacement recovery, got original terminal order: {:?}",
                resolved.order.status
            );
        }
    }
}

#[tokio::test]
async fn orders_mleg_bwb_market_roundtrip_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_mleg_bwb_market_roundtrip_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_bwb_market_roundtrip_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_mleg_bwb_market_roundtrip_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_roll_market_live_paper() {
    let _guard = target_support::lock_live_paper_account().await;
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::LivePaper).await
    else {
        return;
    };
    orders_mleg_roll_market_scenario(&harness).await;
}

#[tokio::test]
async fn orders_mleg_roll_market_mock() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };
    orders_mleg_roll_market_scenario(&harness).await;
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
    let client_order_id =
        unique_client_order_id(&format!("phase12-{}-basic", target_slug(harness)));

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
    let create_limit_price = if harness.is_mock() {
        spread.more_conservative_limit_price
    } else {
        spread.non_marketable_limit_price
    };
    let replacement_limit_price = if harness.is_mock() {
        spread.deep_resting_limit_price
    } else {
        spread.more_conservative_limit_price
    };
    let structure_qty = Decimal::new(2, 0);
    let client_order_id = unique_client_order_id(&format!("phase20-{}-mleg", target_slug(harness)));
    let replaced_client_order_id = format!("{client_order_id}-replaced");

    let created = client
        .orders()
        .create(CreateRequest {
            qty: Some(structure_qty),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(create_limit_price),
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
    assert_eq!(created.qty, Some(structure_qty));
    assert!(
        created
            .legs
            .as_ref()
            .is_some_and(|legs| legs.len() == spread.legs.len())
    );
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
    assert_eq!(fetched.qty, Some(structure_qty));
    assert!(
        fetched
            .legs
            .as_ref()
            .is_some_and(|legs| legs.len() == spread.legs.len())
    );

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("multi-leg client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let replacement = match client
        .orders()
        .replace_resolved(
            &created.id,
            ReplaceRequest {
                qty: None,
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(replacement_limit_price),
                stop_price: None,
                trail: None,
                client_order_id: Some(replaced_client_order_id.clone()),
            },
        )
        .await
        .expect("multi-leg order replace should succeed")
    {
        ReplaceResolution::NewOrder(resolved) => resolved.order,
        ReplaceResolution::OriginalOrderTerminal(resolved) => panic!(
            "multi-leg replace should produce a new order, got terminal original order: {:?}",
            resolved.order.status
        ),
    };
    assert_ne!(replacement.id, created.id);
    assert_eq!(replacement.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(replacement.qty, Some(structure_qty));
    assert_eq!(replacement.limit_price, Some(replacement_limit_price));
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
    assert!(
        replacement_by_client_order_id
            .legs
            .as_ref()
            .is_some_and(|legs| legs.len() == spread.legs.len())
    );

    let nested_list = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(100),
            nested: Some(true),
            ..ListRequest::default()
        })
        .await
        .expect("nested order list should remain readable for multi-leg replacement");
    let nested_replacement = nested_list
        .into_iter()
        .find(|order| order.id == replacement.id)
        .expect("nested order list should include the replacement multi-leg order");
    assert!(
        nested_replacement
            .legs
            .as_ref()
            .is_some_and(|legs| legs.len() == spread.legs.len())
    );

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
    let stock_limit_price =
        non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
            .await
            .expect("non-marketable stock price should be discoverable for cancel_all");
    let spread = discover_mleg_call_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted multi-leg call spread should be discoverable for cancel_all");
    let mleg_limit_price = if harness.is_mock() {
        spread.deep_resting_limit_price
    } else {
        spread.non_marketable_limit_price
    };

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
            limit_price: Some(mleg_limit_price),
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

async fn orders_trailing_stop_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "trailing stop lifecycle").await {
        return;
    }

    ensure_symbol_flat(harness, ORDER_TEST_SYMBOL).await;

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("trailing stop price context should be discoverable");
    let trail_price = trailing_stop_amount(&pricing);
    let replacement_trail_price = (trail_price + Decimal::ONE).round_dp(2);
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let open_order = open_test_equity_position(
            harness,
            ORDER_TEST_SYMBOL,
            &unique_client_order_id(&format!("phase21-{}-trailing-open", target_slug(harness))),
        )
        .await?;
        assert_eq!(open_order.status, OrderStatus::Filled);
        let position = wait_for_position(harness, ORDER_TEST_SYMBOL).await;
        assert_eq!(position.symbol, ORDER_TEST_SYMBOL);

        let client_order_id =
            unique_client_order_id(&format!("phase21-{}-trailing", target_slug(harness)));
        let replaced_client_order_id = format!("{client_order_id}-replaced");
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Sell),
                r#type: Some(OrderType::TrailingStop),
                time_in_force: Some(TimeInForce::Day),
                limit_price: None,
                stop_price: None,
                trail_price: Some(trail_price),
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(client_order_id.clone()),
                ..CreateRequest::default()
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.r#type, OrderType::TrailingStop);
        assert_eq!(created.trail_price, Some(trail_price));
        assert_eq!(created.side, OrderSide::Sell);

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.trail_price, Some(trail_price));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let replacement = client
            .orders()
            .replace(
                &created.id,
                ReplaceRequest {
                    qty: None,
                    time_in_force: Some(TimeInForce::Day),
                    limit_price: None,
                    stop_price: None,
                    trail: Some(replacement_trail_price),
                    client_order_id: Some(replaced_client_order_id.clone()),
                },
            )
            .await?;
        cleanup_order_id = Some(replacement.id.clone());
        assert_ne!(replacement.id, created.id);
        assert_eq!(replacement.replaces.as_deref(), Some(created.id.as_str()));
        assert_eq!(replacement.trail_price, Some(replacement_trail_price));

        let replaced_source =
            wait_for_order_status(harness, &created.id, OrderStatus::Replaced).await?;
        assert_eq!(
            replaced_source.replaced_by.as_deref(),
            Some(replacement.id.as_str())
        );

        let replacement_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&replaced_client_order_id)
            .await?;
        assert_eq!(replacement_by_client_order_id.id, replacement.id);

        let listed_open = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed_open.iter().any(|order| order.id == replacement.id));

        let canceled = cancel_order_and_wait(harness, &replacement.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-trailing-stop-canceled",
            &canceled,
            "trailing stop order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    let _ = wait_for_no_open_orders_for_symbol(harness, ORDER_TEST_SYMBOL).await;
    ensure_symbol_flat(harness, ORDER_TEST_SYMBOL).await;

    result.expect("trailing stop lifecycle should complete against live paper");
}

async fn orders_fractional_qty_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "fractional qty lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("fractional limit price should be discoverable");
    let requested_qty = Decimal::new(25, 2);
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-fractional", target_slug(harness)));
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: Some(requested_qty),
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
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.qty, Some(requested_qty));
        assert_eq!(created.notional, None);

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.qty, Some(requested_qty));

        let listed = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed.iter().any(|order| order.id == created.id));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);
        assert_eq!(fetched_by_client_order_id.qty, Some(requested_qty));

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        assert_eq!(canceled.qty, Some(requested_qty));
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-fractional-canceled",
            &canceled,
            "fractional order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    result.expect("fractional qty lifecycle should complete against live paper");
}

async fn orders_notional_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "notional lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("notional limit price should be discoverable");
    let requested_notional = Decimal::new(10000, 2);
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-notional", target_slug(harness)));
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: None,
                notional: Some(requested_notional),
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
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.notional, Some(requested_notional));
        assert!(
            created.qty.is_none_or(|qty| qty > Decimal::ZERO),
            "notional order qty, when present, should stay positive"
        );

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.notional, Some(requested_notional));

        let listed = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed.iter().any(|order| order.id == created.id));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);
        assert_eq!(
            fetched_by_client_order_id.notional,
            Some(requested_notional)
        );

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        assert_eq!(canceled.notional, Some(requested_notional));
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-notional-canceled",
            &canceled,
            "notional order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    result.expect("notional lifecycle should complete against live paper");
}

async fn orders_extended_hours_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "extended-hours lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let limit_price = non_marketable_buy_limit_price(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("extended-hours limit price should be discoverable");
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-extended-hours", target_slug(harness)));
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
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
                extended_hours: Some(true),
                client_order_id: Some(client_order_id.clone()),
                ..CreateRequest::default()
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert!(created.extended_hours);
        assert_eq!(created.limit_price, Some(limit_price));

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert!(fetched.extended_hours);

        let listed = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed.iter().any(|order| order.id == created.id));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);
        assert!(fetched_by_client_order_id.extended_hours);

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        assert!(canceled.extended_hours);
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-extended-hours-canceled",
            &canceled,
            "extended-hours order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    result.expect("extended-hours lifecycle should complete against live paper");
}

async fn orders_bracket_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "bracket lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("bracket price context should be discoverable");
    let entry_limit_price = advanced_entry_buy_limit_price(&pricing);
    let stop_loss_price = protective_long_stop_price(&pricing, entry_limit_price);
    let take_profit = TakeProfit {
        limit_price: pricing.resting_sell_limit_price,
    };
    let stop_loss = StopLoss {
        stop_price: stop_loss_price,
        limit_price: None,
    };
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-bracket", target_slug(harness)));
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(entry_limit_price),
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(client_order_id.clone()),
                order_class: Some(OrderClass::Bracket),
                take_profit: Some(take_profit.clone()),
                stop_loss: Some(stop_loss.clone()),
                legs: None,
                position_intent: None,
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.order_class, OrderClass::Bracket);
        assert_eq!(created.limit_price, Some(entry_limit_price));
        assert_eq!(created.side, OrderSide::Buy);

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.order_class, OrderClass::Bracket);

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let nested_list = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::All),
                limit: Some(100),
                nested: Some(true),
                ..ListRequest::default()
            })
            .await?;
        let nested = nested_list
            .into_iter()
            .find(|order| order.id == created.id)
            .expect("nested orders list should expose the bracket order");
        let nested_legs = nested
            .legs
            .expect("nested bracket order should include exit legs");
        assert_eq!(nested_legs.len(), 2);
        assert!(nested_legs.iter().all(|leg| leg.side == OrderSide::Sell));

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-bracket-canceled",
            &canceled,
            "bracket order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    result.expect("bracket lifecycle should complete against live paper");
}

async fn orders_oto_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "oto lifecycle").await {
        return;
    }

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("oto price context should be discoverable");
    let entry_limit_price = advanced_entry_buy_limit_price(&pricing);
    let take_profit = TakeProfit {
        limit_price: pricing.resting_sell_limit_price,
    };
    let client_order_id = unique_client_order_id(&format!("phase21-{}-oto", target_slug(harness)));
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(entry_limit_price),
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(client_order_id.clone()),
                order_class: Some(OrderClass::Oto),
                take_profit: Some(take_profit.clone()),
                stop_loss: None,
                legs: None,
                position_intent: None,
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.order_class, OrderClass::Oto);
        assert_eq!(created.limit_price, Some(entry_limit_price));
        assert_eq!(created.side, OrderSide::Buy);
        assert_eq!(created.stop_loss, None);

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.order_class, OrderClass::Oto);

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let nested_list = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::All),
                limit: Some(100),
                nested: Some(true),
                ..ListRequest::default()
            })
            .await?;
        let nested = nested_list
            .into_iter()
            .find(|order| order.id == created.id)
            .expect("nested orders list should expose the oto order");
        let nested_legs = nested
            .legs
            .expect("nested oto order should include one exit leg");
        assert_eq!(nested_legs.len(), 1);
        assert_eq!(nested_legs[0].side, OrderSide::Sell);

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-oto-canceled",
            &canceled,
            "oto order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    result.expect("oto lifecycle should complete against live paper");
}

async fn orders_oco_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "oco lifecycle").await {
        return;
    }

    ensure_symbol_flat(harness, ORDER_TEST_SYMBOL).await;

    let client = harness.trade_client();
    let pricing = stock_order_price_context(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("oco price context should be discoverable");
    let take_profit = TakeProfit {
        limit_price: pricing.resting_sell_limit_price,
    };
    let stop_loss = StopLoss {
        stop_price: protective_long_stop_price(&pricing, pricing.bid.round_dp(2)),
        limit_price: None,
    };
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let open_order = open_test_equity_position(
            harness,
            ORDER_TEST_SYMBOL,
            &unique_client_order_id(&format!("phase21-{}-oco-open", target_slug(harness))),
        )
        .await?;
        assert_eq!(open_order.status, OrderStatus::Filled);
        let position = wait_for_position(harness, ORDER_TEST_SYMBOL).await;
        assert_eq!(position.symbol, ORDER_TEST_SYMBOL);

        let client_order_id =
            unique_client_order_id(&format!("phase21-{}-oco", target_slug(harness)));
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Sell),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: None,
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(client_order_id.clone()),
                order_class: Some(OrderClass::Oco),
                take_profit: Some(take_profit.clone()),
                stop_loss: Some(stop_loss.clone()),
                legs: None,
                position_intent: None,
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.order_class, OrderClass::Oco);
        assert_eq!(created.side, OrderSide::Sell);

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.order_class, OrderClass::Oco);

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let nested_list = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                nested: Some(true),
                ..ListRequest::default()
            })
            .await?;
        let nested = nested_list
            .into_iter()
            .find(|order| order.id == created.id)
            .expect("nested orders list should expose the oco order");
        let nested_legs = nested
            .legs
            .expect("nested oco order should include a stop-loss child leg");
        assert_eq!(nested_legs.len(), 1);
        assert_eq!(nested_legs[0].side, OrderSide::Sell);

        let canceled = cancel_order_and_wait(harness, &created.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "paper-oco-canceled",
            &canceled,
            "oco order canceled sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    let _ = wait_for_no_open_orders_for_symbol(harness, ORDER_TEST_SYMBOL).await;
    ensure_symbol_flat(harness, ORDER_TEST_SYMBOL).await;

    result.expect("oco lifecycle should complete against live paper");
}

async fn orders_option_limit_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "option limit lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let contract = discover_single_leg_call(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted single-leg option contract should be discoverable");
    ensure_symbol_flat(harness, &contract.contract_symbol).await;

    let client = harness.trade_client();
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-option-limit", target_slug(harness)));
    let replaced_client_order_id = format!("{client_order_id}-replaced");
    let mut cleanup_order_id: Option<String> = None;

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some(contract.contract_symbol.clone()),
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(contract.non_marketable_limit_price),
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(client_order_id.clone()),
                order_class: None,
                take_profit: None,
                stop_loss: None,
                legs: None,
                position_intent: Some(PositionIntent::BuyToOpen),
            })
            .await?;
        cleanup_order_id = Some(created.id.clone());
        assert_eq!(created.symbol, contract.contract_symbol);
        assert_eq!(created.asset_class, "us_option");
        assert_eq!(created.position_intent, Some(PositionIntent::BuyToOpen));
        assert_eq!(
            created.limit_price,
            Some(contract.non_marketable_limit_price)
        );

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.asset_class, "us_option");

        let listed = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                symbols: Some(vec![contract.contract_symbol.clone()]),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed.iter().any(|order| order.id == created.id));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let replacement = match client
            .orders()
            .replace_resolved(
                &created.id,
                ReplaceRequest {
                    qty: None,
                    time_in_force: Some(TimeInForce::Day),
                    limit_price: Some(contract.more_conservative_limit_price),
                    stop_price: None,
                    trail: None,
                    client_order_id: Some(replaced_client_order_id.clone()),
                },
            )
            .await?
        {
            ReplaceResolution::NewOrder(resolved) => resolved.order,
            ReplaceResolution::OriginalOrderTerminal(resolved) => {
                return Err(Error::InvalidRequest(format!(
                    "option limit replace returned terminal original order: {:?}",
                    resolved.order.status
                )));
            }
        };
        cleanup_order_id = Some(replacement.id.clone());
        assert_ne!(replacement.id, created.id);
        assert_eq!(replacement.replaces.as_deref(), Some(created.id.as_str()));
        assert_eq!(
            replacement.limit_price,
            Some(contract.more_conservative_limit_price)
        );

        let replaced_source =
            wait_for_order_status(harness, &created.id, OrderStatus::Replaced).await?;
        assert_eq!(
            replaced_source.replaced_by.as_deref(),
            Some(replacement.id.as_str())
        );

        let replacement_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&replaced_client_order_id)
            .await?;
        assert_eq!(replacement_by_client_order_id.id, replacement.id);

        let canceled = cancel_order_and_wait(harness, &replacement.id).await?;
        cleanup_order_id = None;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        assert_eq!(canceled.asset_class, "us_option");
        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "option-limit-canceled",
            &canceled,
            "single-leg option limit sample should record",
        );

        Ok(())
    }
    .await;

    maybe_cancel_order(harness, cleanup_order_id.as_deref()).await;
    ensure_symbol_flat(harness, &contract.contract_symbol).await;

    result.expect("single-leg option limit lifecycle should complete against live paper");
}

async fn orders_option_market_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "option market lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let contract = discover_single_leg_call(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted single-leg option contract should be discoverable");
    ensure_symbol_flat(harness, &contract.contract_symbol).await;

    let result: Result<(), Error> = async {
        let opened = submit_option_market_order(
            harness,
            &contract.contract_symbol,
            Decimal::ONE,
            OrderSide::Buy,
            PositionIntent::BuyToOpen,
            &unique_client_order_id(&format!(
                "phase21-{}-option-market-open",
                target_slug(harness)
            )),
        )
        .await?;
        assert_eq!(opened.status, OrderStatus::Filled);
        assert_eq!(opened.asset_class, "us_option");
        assert_eq!(opened.position_intent, Some(PositionIntent::BuyToOpen));

        let opened_position = wait_for_position(harness, &contract.contract_symbol).await;
        assert_eq!(opened_position.symbol, contract.contract_symbol);
        assert_eq!(opened_position.asset_class, "us_option");

        let closed = submit_option_market_order(
            harness,
            &contract.contract_symbol,
            Decimal::ONE,
            OrderSide::Sell,
            PositionIntent::SellToClose,
            &unique_client_order_id(&format!(
                "phase21-{}-option-market-close",
                target_slug(harness)
            )),
        )
        .await?;
        assert_eq!(closed.status, OrderStatus::Filled);
        assert_eq!(closed.position_intent, Some(PositionIntent::SellToClose));
        wait_for_position_absent(harness, &contract.contract_symbol).await;

        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "option-market-roundtrip",
            &vec![opened.clone(), closed.clone()],
            "single-leg option market roundtrip should record",
        );

        Ok(())
    }
    .await;

    ensure_symbol_flat(harness, &contract.contract_symbol).await;
    result.expect("single-leg option market lifecycle should complete against live paper");
}

async fn orders_mleg_market_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "multi-leg market lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let spread = discover_mleg_put_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted multi-leg put spread should be discoverable");
    for leg in &spread.legs {
        ensure_symbol_flat(harness, &leg.symbol).await;
    }

    let client = harness.trade_client();
    let client_order_id =
        unique_client_order_id(&format!("phase21-{}-mleg-market", target_slug(harness)));
    let structure_qty = Decimal::new(2, 0);

    let result: Result<(), Error> = async {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: None,
                qty: Some(structure_qty),
                notional: None,
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                limit_price: None,
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
            })
            .await?;
        assert_eq!(created.order_class, OrderClass::Mleg);
        assert_eq!(created.r#type, OrderType::Market);
        assert_eq!(created.qty, Some(structure_qty));
        assert!(
            created
                .legs
                .as_ref()
                .is_some_and(|legs| legs.len() == spread.legs.len())
        );

        let fetched = client.orders().get(&created.id).await?;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.order_class, OrderClass::Mleg);
        assert!(
            fetched
                .legs
                .as_ref()
                .is_some_and(|legs| legs.len() == spread.legs.len())
        );

        let listed = client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::All),
                limit: Some(100),
                nested: Some(true),
                ..ListRequest::default()
            })
            .await?;
        assert!(listed.iter().any(|order| order.id == created.id));

        let fetched_by_client_order_id = client
            .orders()
            .get_by_client_order_id(&client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, created.id);

        let filled = wait_for_order_status(harness, &created.id, OrderStatus::Filled).await?;
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.order_class, OrderClass::Mleg);
        assert_eq!(filled.qty, Some(structure_qty));
        let filled_legs = filled
            .legs
            .as_ref()
            .expect("filled multi-leg order should retain nested legs");
        assert_eq!(filled_legs.len(), spread.legs.len());
        assert!(
            filled_legs
                .iter()
                .all(|leg| leg.status == OrderStatus::Filled && leg.filled_avg_price.is_some())
        );

        assert_positions_match_mleg_open(harness, &spread.legs, 2).await;

        close_filled_mleg_legs(harness, &filled, "phase21").await?;

        for leg in &spread.legs {
            wait_for_position_absent(harness, &leg.symbol).await;
        }

        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "mleg-market-roundtrip",
            &filled,
            "multi-leg market roundtrip should record",
        );

        Ok(())
    }
    .await;

    for leg in &spread.legs {
        ensure_symbol_flat(harness, &leg.symbol).await;
    }

    result.expect("multi-leg option market lifecycle should complete against live paper");
}

async fn orders_mleg_bwb_market_roundtrip_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "multi-leg 1:2:1 market roundtrip").await {
        return;
    }

    clear_option_universe_cache().await;
    let bwb = discover_mleg_call_broken_wing_butterfly(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("quoted 1:2:1 call structure should be discoverable");
    for leg in &bwb.legs {
        ensure_symbol_flat(harness, &leg.symbol).await;
    }

    let structure_qty = Decimal::new(2, 0);

    let result: Result<(), Error> = async {
        let opened = submit_mleg_market_order(
            harness,
            bwb.legs.clone(),
            structure_qty,
            OrderSide::Buy,
            &unique_client_order_id(&format!("phase21-{}-bwb-market-open", target_slug(harness))),
        )
        .await?;
        let opened = wait_for_order_status(harness, &opened.id, OrderStatus::Filled).await?;
        assert_mleg_filled(&opened, bwb.legs.len());
        assert_positions_match_mleg_open(harness, &bwb.legs, 2).await;

        let closed = submit_mleg_market_order(
            harness,
            reverse_option_legs(&bwb.legs),
            structure_qty,
            OrderSide::Sell,
            &unique_client_order_id(&format!(
                "phase21-{}-bwb-market-close",
                target_slug(harness)
            )),
        )
        .await?;
        let closed = wait_for_order_status(harness, &closed.id, OrderStatus::Filled).await?;
        assert_mleg_filled(&closed, bwb.legs.len());

        for leg in &bwb.legs {
            wait_for_position_absent(harness, &leg.symbol).await;
        }

        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "mleg-bwb-market-roundtrip",
            &vec![opened.clone(), closed.clone()],
            "multi-leg 1:2:1 market roundtrip should record",
        );

        Ok(())
    }
    .await;

    for leg in &bwb.legs {
        ensure_symbol_flat(harness, &leg.symbol).await;
    }

    result.expect("multi-leg 1:2:1 market lifecycle should complete against live paper");
}

async fn orders_mleg_roll_market_scenario(_harness: &TradeTestHarness) {
    let harness = _harness;
    if maybe_skip_live_market_session(harness, "multi-leg roll market lifecycle").await {
        return;
    }

    clear_option_universe_cache().await;
    let (opened_spread, replacement_spread) =
        discover_distinct_mleg_call_spread_pair(harness.data_client(), ORDER_TEST_SYMBOL)
            .await
            .expect("two quoted call spreads should be discoverable for a roll");

    for symbol in opened_spread
        .legs
        .iter()
        .chain(replacement_spread.legs.iter())
        .map(|leg| leg.symbol.as_str())
    {
        ensure_symbol_flat(harness, symbol).await;
    }

    let structure_qty = Decimal::new(2, 0);

    let result: Result<(), Error> = async {
        let opened = submit_mleg_market_order(
            harness,
            opened_spread.legs.clone(),
            structure_qty,
            OrderSide::Buy,
            &unique_client_order_id(&format!("phase21-{}-mleg-roll-open", target_slug(harness))),
        )
        .await?;
        let opened = wait_for_order_status(harness, &opened.id, OrderStatus::Filled).await?;
        assert_mleg_filled(&opened, opened_spread.legs.len());
        assert_positions_match_mleg_open(harness, &opened_spread.legs, 2).await;

        let roll_legs = build_roll_option_legs(&opened_spread.legs, &replacement_spread.legs);
        let rolled = submit_mleg_market_order(
            harness,
            roll_legs,
            structure_qty,
            OrderSide::Sell,
            &unique_client_order_id(&format!(
                "phase21-{}-mleg-roll-rotate",
                target_slug(harness)
            )),
        )
        .await?;
        let rolled = wait_for_order_status(harness, &rolled.id, OrderStatus::Filled).await?;
        assert_mleg_filled(
            &rolled,
            opened_spread.legs.len() + replacement_spread.legs.len(),
        );

        for leg in &opened_spread.legs {
            wait_for_position_absent(harness, &leg.symbol).await;
        }
        assert_positions_match_mleg_open(harness, &replacement_spread.legs, 2).await;

        let closed = submit_mleg_market_order(
            harness,
            reverse_option_legs(&replacement_spread.legs),
            structure_qty,
            OrderSide::Sell,
            &unique_client_order_id(&format!("phase21-{}-mleg-roll-close", target_slug(harness))),
        )
        .await?;
        let closed = wait_for_order_status(harness, &closed.id, OrderStatus::Filled).await?;
        assert_mleg_filled(&closed, replacement_spread.legs.len());

        for leg in &replacement_spread.legs {
            wait_for_position_absent(harness, &leg.symbol).await;
        }

        maybe_record_live_json(
            harness,
            "alpaca-trade-orders",
            "mleg-roll-market-roundtrip",
            &vec![opened.clone(), rolled.clone(), closed.clone()],
            "multi-leg roll market roundtrip should record",
        );

        Ok(())
    }
    .await;

    for symbol in opened_spread
        .legs
        .iter()
        .chain(replacement_spread.legs.iter())
        .map(|leg| leg.symbol.as_str())
    {
        ensure_symbol_flat(harness, symbol).await;
    }

    result.expect("multi-leg roll lifecycle should complete against live paper");
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
        recorder.record_json(suite, name, payload).expect(context);
    }
}

fn target_slug(harness: &TradeTestHarness) -> &'static str {
    if harness.is_mock() { "mock" } else { "paper" }
}

fn advanced_entry_buy_limit_price(pricing: &StockOrderPriceContext) -> Decimal {
    let minimum_tick = Decimal::new(1, 2);
    let candidate = (pricing.bid * Decimal::new(95, 2)).round_dp(2);
    if candidate > minimum_tick {
        candidate
    } else {
        minimum_tick
    }
}

fn protective_long_stop_price(
    pricing: &StockOrderPriceContext,
    entry_limit_price: Decimal,
) -> Decimal {
    let minimum_tick = Decimal::new(1, 2);
    let market_candidate = (pricing.bid * Decimal::new(95, 2)).round_dp(2);
    let entry_guard = (entry_limit_price - Decimal::new(50, 2)).round_dp(2);
    let bounded = market_candidate.min(entry_guard);
    if bounded > minimum_tick {
        bounded
    } else {
        minimum_tick
    }
}

fn trailing_stop_amount(pricing: &StockOrderPriceContext) -> Decimal {
    let minimum = Decimal::new(500, 2);
    let candidate = (pricing.ask * Decimal::new(2, 2)).round_dp(2);
    candidate.max(minimum)
}

async fn open_test_equity_position(
    harness: &TradeTestHarness,
    symbol: &str,
    client_order_id: &str,
) -> Result<alpaca_trade::orders::Order, Error> {
    let opened = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
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
            client_order_id: Some(client_order_id.to_owned()),
            ..CreateRequest::default()
        })
        .await?;

    wait_for_order_status(harness, &opened.id, OrderStatus::Filled).await
}

async fn submit_option_market_order(
    harness: &TradeTestHarness,
    symbol: &str,
    qty: Decimal,
    side: OrderSide,
    position_intent: PositionIntent,
    client_order_id: &str,
) -> Result<alpaca_trade::orders::Order, Error> {
    let created = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
            qty: Some(qty),
            notional: None,
            side: Some(side),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            limit_price: None,
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.to_owned()),
            order_class: None,
            take_profit: None,
            stop_loss: None,
            legs: None,
            position_intent: Some(position_intent),
        })
        .await?;

    wait_for_order_status(harness, &created.id, OrderStatus::Filled).await
}

async fn submit_mleg_market_order(
    harness: &TradeTestHarness,
    legs: Vec<OptionLegRequest>,
    qty: Decimal,
    side: OrderSide,
    client_order_id: &str,
) -> Result<alpaca_trade::orders::Order, Error> {
    let created = harness
        .trade_client()
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(qty),
            notional: None,
            side: Some(side),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            limit_price: None,
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.to_owned()),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(legs),
            position_intent: None,
        })
        .await?;

    wait_for_order_status(harness, &created.id, OrderStatus::Filled).await
}

async fn close_filled_mleg_legs(
    harness: &TradeTestHarness,
    filled_order: &alpaca_trade::orders::Order,
    phase_slug: &str,
) -> Result<(), Error> {
    let structure_qty = filled_order.qty.unwrap_or(Decimal::ONE);
    let legs = filled_order
        .legs
        .as_ref()
        .expect("filled multi-leg order should expose nested legs");
    for (index, leg) in legs.iter().enumerate() {
        let (close_side, close_intent) = reverse_leg_close_shape(
            leg.side.clone(),
            leg.position_intent
                .clone()
                .expect("filled multi-leg leg should expose position intent"),
        );
        let close_qty = structure_qty * leg.ratio_qty.map(Decimal::from).unwrap_or(Decimal::ONE);
        let closed = submit_option_market_order(
            harness,
            &leg.symbol,
            close_qty,
            close_side,
            close_intent,
            &unique_client_order_id(&format!(
                "{phase_slug}-{}-mleg-close-leg-{index}",
                target_slug(harness)
            )),
        )
        .await?;
        assert_eq!(closed.status, OrderStatus::Filled);
    }

    Ok(())
}

async fn build_close_option_legs(
    harness: &TradeTestHarness,
    filled_order: &alpaca_trade::orders::Order,
) -> Vec<CloseOptionLeg> {
    let legs = filled_order
        .legs
        .as_ref()
        .expect("filled multi-leg order should expose nested legs");
    let symbols = legs
        .iter()
        .map(|leg| leg.symbol.clone())
        .collect::<Vec<_>>();
    let quotes = load_option_quotes(harness, &symbols).await;

    legs.iter()
        .map(|leg| {
            let (close_side, close_intent) = reverse_leg_close_shape(
                leg.side,
                leg.position_intent
                    .expect("filled multi-leg leg should expose position intent"),
            );
            CloseOptionLeg {
                symbol: leg.symbol.clone(),
                ratio_qty: leg.ratio_qty.unwrap_or(1),
                side: close_side,
                position_intent: close_intent,
                quote: quotes.get(&leg.symbol).cloned(),
            }
        })
        .collect()
}

async fn load_option_quotes(
    harness: &TradeTestHarness,
    symbols: &[String],
) -> std::collections::HashMap<String, OptionQuote> {
    let snapshots = harness
        .data_client()
        .options()
        .snapshots(SnapshotsRequest {
            symbols: symbols.to_vec(),
            feed: Some(OptionsFeed::Indicative),
            limit: Some(symbols.len() as u32),
            page_token: None,
        })
        .await
        .expect("option snapshots should load for close-option-legs tests");

    snapshots
        .snapshots
        .into_iter()
        .filter_map(|(symbol, snapshot)| {
            let quote = snapshot.latest_quote?;
            let bid = quote.bp.or(quote.ap)?;
            let ask = quote.ap.or(quote.bp)?;
            Some((symbol, OptionQuote { bid, ask }))
        })
        .collect()
}

fn reverse_leg_close_shape(
    open_side: OrderSide,
    open_intent: PositionIntent,
) -> (OrderSide, PositionIntent) {
    match (open_side, open_intent) {
        (OrderSide::Buy, PositionIntent::BuyToOpen) => {
            (OrderSide::Sell, PositionIntent::SellToClose)
        }
        (OrderSide::Sell, PositionIntent::SellToOpen) => {
            (OrderSide::Buy, PositionIntent::BuyToClose)
        }
        (side, intent) => panic!("unexpected multi-leg open shape: {side:?} / {intent:?}"),
    }
}

fn reverse_option_leg(leg: &OptionLegRequest) -> OptionLegRequest {
    let (close_side, close_intent) = reverse_leg_close_shape(
        leg.side
            .clone()
            .expect("multi-leg request leg should include side"),
        leg.position_intent
            .clone()
            .expect("multi-leg request leg should include position intent"),
    );

    OptionLegRequest {
        symbol: leg.symbol.clone(),
        ratio_qty: leg.ratio_qty,
        side: Some(close_side),
        position_intent: Some(close_intent),
    }
}

fn reverse_option_legs(legs: &[OptionLegRequest]) -> Vec<OptionLegRequest> {
    legs.iter().map(reverse_option_leg).collect()
}

fn build_roll_option_legs(
    closing_legs: &[OptionLegRequest],
    opening_legs: &[OptionLegRequest],
) -> Vec<OptionLegRequest> {
    let mut legs = reverse_option_legs(closing_legs);
    legs.extend(opening_legs.iter().cloned());
    legs
}

fn assert_mleg_filled(order: &alpaca_trade::orders::Order, expected_leg_count: usize) {
    assert_eq!(order.status, OrderStatus::Filled);
    assert_eq!(order.order_class, OrderClass::Mleg);
    let legs = order
        .legs
        .as_ref()
        .expect("filled multi-leg order should retain nested legs");
    assert_eq!(legs.len(), expected_leg_count);
    assert!(
        legs.iter()
            .all(|leg| leg.status == OrderStatus::Filled && leg.filled_avg_price.is_some())
    );
}

async fn assert_positions_match_mleg_open(
    harness: &TradeTestHarness,
    legs: &[OptionLegRequest],
    structure_qty: i32,
) {
    for leg in legs {
        let position = wait_for_position(harness, &leg.symbol).await;
        assert_eq!(position.symbol, leg.symbol);
        assert_eq!(position.asset_class, "us_option");
        let expected_qty = match leg
            .side
            .clone()
            .expect("multi-leg request leg should include side")
        {
            OrderSide::Buy => Decimal::from(i64::from(leg.ratio_qty) * i64::from(structure_qty)),
            OrderSide::Sell => -Decimal::from(i64::from(leg.ratio_qty) * i64::from(structure_qty)),
            OrderSide::Unspecified => panic!("multi-leg request leg used unspecified side"),
        };
        assert_eq!(position.qty, expected_qty);
        let expected_side = match leg
            .side
            .clone()
            .expect("multi-leg request leg should include side")
        {
            OrderSide::Buy => "long",
            OrderSide::Sell => "short",
            OrderSide::Unspecified => panic!("multi-leg request leg used unspecified side"),
        };
        assert_eq!(position.side, expected_side);
    }
}

async fn cancel_order_and_wait(
    harness: &TradeTestHarness,
    order_id: &str,
) -> Result<alpaca_trade::orders::Order, Error> {
    harness
        .trade_client()
        .orders()
        .cancel_resolved(order_id)
        .await
        .map(|resolved| resolved.order)
}

async fn maybe_cancel_order(harness: &TradeTestHarness, order_id: Option<&str>) {
    if let Some(order_id) = order_id {
        let _ = harness
            .trade_client()
            .orders()
            .cancel_resolved(order_id)
            .await;
    }
}

async fn wait_for_no_open_orders_for_symbol(
    harness: &TradeTestHarness,
    symbol: &str,
) -> Result<(), Error> {
    for _attempt in 0..harness.poll_attempts() {
        let open_orders = harness
            .trade_client()
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(100),
                symbols: Some(vec![symbol.to_owned()]),
                ..ListRequest::default()
            })
            .await?;
        if open_orders.is_empty() {
            return Ok(());
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    let open_orders = harness
        .trade_client()
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(100),
            symbols: Some(vec![symbol.to_owned()]),
            ..ListRequest::default()
        })
        .await?;
    if open_orders.is_empty() {
        Ok(())
    } else {
        Err(Error::InvalidRequest(format!(
            "open orders still remain for {symbol}"
        )))
    }
}

async fn wait_for_order_status(
    harness: &TradeTestHarness,
    order_id: &str,
    expected_status: OrderStatus,
) -> Result<alpaca_trade::orders::Order, Error> {
    harness
        .trade_client()
        .orders()
        .wait_for(order_id, WaitFor::Exact(expected_status))
        .await
}

async fn build_recovery_test_client() -> Option<(TestServer, MockServerState, TradeClient)> {
    let env = live_support::LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(live_support::AlpacaService::Data) {
        eprintln!("skipping mock recovery test: {reason}");
        return None;
    }

    let data_service = env.data().expect("data service should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("alpaca-data client should build");
    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client));
    let server = spawn_test_server_with_state(state.clone()).await;
    let client = TradeClient::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    Some((server, state, client))
}

async fn create_non_marketable_spy_limit_order(
    client: &TradeClient,
) -> alpaca_trade::orders::Order {
    client
        .orders()
        .create(CreateRequest {
            symbol: Some(ORDER_TEST_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::ONE),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(unique_client_order_id("recovery-open")),
            order_class: None,
            take_profit: None,
            stop_loss: None,
            legs: None,
            position_intent: None,
        })
        .await
        .expect("non-marketable recovery order should submit")
}
