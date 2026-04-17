#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;
#[path = "support/targets.rs"]
mod target_support;

use alpaca_trade::orders::{
    CreateRequest, OrderClass, OrderSide, OrderStatus, OrderType, TimeInForce,
};
use order_support::{
    clear_option_universe_cache, discover_mleg_call_broken_wing_butterfly,
    discover_mleg_iron_condor, discover_mleg_put_spread, unique_client_order_id,
};
use rust_decimal::Decimal;
use target_support::TradeTestTarget;

const ORDER_TEST_SYMBOL: &str = "SPY";

#[tokio::test]
async fn orders_mock_market_multi_leg_orders_fill_for_strategy_shapes() {
    let Some(harness) = target_support::build_trade_test_harness(TradeTestTarget::Mock).await
    else {
        return;
    };

    clear_option_universe_cache().await;
    let put_spread = discover_mleg_put_spread(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("dynamic put spread should be discoverable");
    clear_option_universe_cache().await;
    let bwb = discover_mleg_call_broken_wing_butterfly(harness.data_client(), ORDER_TEST_SYMBOL)
        .await
        .expect("dynamic broken wing butterfly should be discoverable");
    clear_option_universe_cache().await;
    let maybe_iron_condor =
        discover_mleg_iron_condor(harness.data_client(), ORDER_TEST_SYMBOL).await;
    let client = harness.trade_client();

    let mut strategies = vec![("put-spread", put_spread), ("bwb", bwb)];
    match maybe_iron_condor {
        Ok(iron_condor) => strategies.push(("iron-condor", iron_condor)),
        Err(reason) => eprintln!("skipping mock iron condor subcase: {reason}"),
    }

    for (name, strategy) in strategies {
        let filled = client
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
                client_order_id: Some(unique_client_order_id(&format!(
                    "phase20-mock-contract-{name}"
                ))),
                order_class: Some(OrderClass::Mleg),
                take_profit: None,
                stop_loss: None,
                legs: Some(strategy.legs.clone()),
                position_intent: None,
            })
            .await
            .expect("mock market multi-leg order create should succeed");
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.order_class, OrderClass::Mleg);
        assert_eq!(filled.r#type, OrderType::Market);
        assert_eq!(filled.qty, Some(Decimal::new(2, 0)));
        assert!(filled.filled_avg_price.is_some());
        let filled_legs = filled
            .legs
            .expect("filled multi-leg order should keep nested legs");
        assert_eq!(filled_legs.len(), strategy.legs.len());
        assert!(
            filled_legs
                .iter()
                .all(|leg| leg.status == OrderStatus::Filled)
        );
        assert!(filled_legs.iter().all(|leg| leg.filled_avg_price.is_some()));
    }
}
