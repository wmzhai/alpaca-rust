use std::collections::HashSet;

use alpaca_mock::state::{CreateOrderInput, ReplaceOrderInput};
use alpaca_mock::{InstrumentSnapshot, MockServerState};
use alpaca_trade::orders::{
    OptionLegRequest, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent, TimeInForce,
};
use rust_decimal::Decimal;
use uuid::{Uuid, Version};

const LONG_CALL: &str = "IWM261218C00200000";
const SHORT_CALL: &str = "IWM261218C00210000";

#[tokio::test]
async fn order_ids_are_globally_unique_across_virtual_accounts_and_replacements() {
    let state = MockServerState::new()
        .with_market_snapshot(
            LONG_CALL,
            InstrumentSnapshot::option(Decimal::new(100, 2), Decimal::new(110, 2)),
        )
        .with_market_snapshot(
            SHORT_CALL,
            InstrumentSnapshot::option(Decimal::new(50, 2), Decimal::new(60, 2)),
        );

    let (created_a, created_b) = tokio::join!(
        state.create_order("mock-key-a", mleg_input("client-a")),
        state.create_order("mock-key-b", mleg_input("client-b")),
    );
    let created_a = created_a.expect("account A order should be created");
    let created_b = created_b.expect("account B order should be created");
    assert_eq!(created_a.status, OrderStatus::New);
    assert_eq!(created_b.status, OrderStatus::New);

    let replacement = ReplaceOrderInput {
        limit_price: Some(Decimal::new(2, 2)),
        ..ReplaceOrderInput::default()
    };
    let (replaced_a, replaced_b) = tokio::join!(
        state.replace_order("mock-key-a", &created_a.id, replacement.clone()),
        state.replace_order("mock-key-b", &created_b.id, replacement),
    );
    let replaced_a = replaced_a.expect("account A order should be replaced");
    let replaced_b = replaced_b.expect("account B order should be replaced");

    assert_eq!(replaced_a.replaces.as_deref(), Some(created_a.id.as_str()));
    assert_eq!(replaced_b.replaces.as_deref(), Some(created_b.id.as_str()));

    let mut seen = HashSet::new();
    for order in [&created_a, &created_b, &replaced_a, &replaced_b] {
        let legs = order
            .legs
            .as_ref()
            .expect("MLEG order should expose nested legs");
        assert_eq!(legs.len(), 2);

        for id in std::iter::once(order.id.as_str()).chain(legs.iter().map(|leg| leg.id.as_str())) {
            let parsed = Uuid::parse_str(id)
                .unwrap_or_else(|error| panic!("{id} should be a UUID: {error}"));
            assert_eq!(parsed.get_version(), Some(Version::Random));
            assert!(seen.insert(id), "duplicate broker order ID: {id}");
        }
    }
}

fn mleg_input(client_order_id: &str) -> CreateOrderInput {
    CreateOrderInput {
        qty: Some(Decimal::ONE),
        order_type: Some(OrderType::Limit),
        time_in_force: Some(TimeInForce::Day),
        limit_price: Some(Decimal::new(1, 2)),
        client_order_id: Some(client_order_id.to_owned()),
        order_class: Some(OrderClass::Mleg),
        legs: Some(vec![
            OptionLegRequest {
                symbol: LONG_CALL.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Buy),
                position_intent: Some(PositionIntent::BuyToOpen),
            },
            OptionLegRequest {
                symbol: SHORT_CALL.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Sell),
                position_intent: Some(PositionIntent::SellToOpen),
            },
        ]),
        ..CreateOrderInput::default()
    }
}
