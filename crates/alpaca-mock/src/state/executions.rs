use rust_decimal::Decimal;

use alpaca_trade::orders::{OrderSide, PositionIntent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionFact {
    pub(crate) sequence: u64,
    pub(crate) order_id: String,
    pub(crate) asset_id: String,
    pub(crate) symbol: String,
    pub(crate) asset_class: String,
    pub(crate) side: OrderSide,
    pub(crate) position_intent: Option<PositionIntent>,
    pub(crate) qty: Decimal,
    pub(crate) price: Decimal,
    pub(crate) occurred_at: String,
}

impl ExecutionFact {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        sequence: u64,
        order_id: String,
        asset_id: String,
        symbol: String,
        asset_class: String,
        side: OrderSide,
        position_intent: Option<PositionIntent>,
        qty: Decimal,
        price: Decimal,
        occurred_at: String,
    ) -> Self {
        Self {
            sequence,
            order_id,
            asset_id,
            symbol,
            asset_class,
            side,
            position_intent,
            qty,
            price,
            occurred_at,
        }
    }
}
