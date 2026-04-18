use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::Error;

use super::{
    CreateRequest, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    PositionIntent, ReplaceRequest, TimeInForce,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubmitOrderStyle {
    Market,
    Limit { limit_price: Decimal },
}

impl OrderSide {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Unspecified => "",
        }
    }
}

impl OrderType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::Stop => "stop",
            Self::StopLimit => "stop_limit",
            Self::TrailingStop => "trailing_stop",
            Self::Unspecified => "",
        }
    }
}

impl TimeInForce {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Gtc => "gtc",
            Self::Opg => "opg",
            Self::Cls => "cls",
            Self::Ioc => "ioc",
            Self::Fok => "fok",
            Self::Gtd => "gtd",
        }
    }
}

impl PositionIntent {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BuyToOpen => "buy_to_open",
            Self::BuyToClose => "buy_to_close",
            Self::SellToOpen => "sell_to_open",
            Self::SellToClose => "sell_to_close",
        }
    }
}

impl OrderClass {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Bracket => "bracket",
            Self::Oco => "oco",
            Self::Oto => "oto",
            Self::Mleg => "mleg",
        }
    }
}

impl OrderStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::PartiallyFilled => "partially_filled",
            Self::Filled => "filled",
            Self::DoneForDay => "done_for_day",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
            Self::Replaced => "replaced",
            Self::PendingCancel => "pending_cancel",
            Self::PendingReplace => "pending_replace",
            Self::Accepted => "accepted",
            Self::PendingNew => "pending_new",
            Self::AcceptedForBidding => "accepted_for_bidding",
            Self::Stopped => "stopped",
            Self::Rejected => "rejected",
            Self::Suspended => "suspended",
            Self::Calculated => "calculated",
            Self::Held => "held",
        }
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled
                | Self::DoneForDay
                | Self::Canceled
                | Self::Expired
                | Self::Rejected
                | Self::Suspended
                | Self::Calculated
        )
    }

    #[must_use]
    pub fn is_stable(self) -> bool {
        matches!(
            self,
            Self::Accepted
                | Self::New
                | Self::Filled
                | Self::Canceled
                | Self::Expired
                | Self::Rejected
        )
    }

    #[must_use]
    pub fn is_cancel_complete(self) -> bool {
        matches!(
            self,
            Self::Canceled | Self::Filled | Self::Expired | Self::Rejected
        )
    }

    #[must_use]
    pub fn is_replace_recovery_terminal(self) -> bool {
        self.is_terminal()
    }
}

impl Order {
    #[must_use]
    pub fn qty_i32(&self) -> Option<i32> {
        self.qty.and_then(|value| value.trunc().to_i32())
    }

    #[must_use]
    pub fn filled_qty_i32(&self) -> i32 {
        self.filled_qty.trunc().to_i32().unwrap_or(0)
    }
}

impl SubmitOrderStyle {
    #[must_use]
    pub fn order_type(self) -> OrderType {
        match self {
            Self::Market => OrderType::Market,
            Self::Limit { .. } => OrderType::Limit,
        }
    }

    #[must_use]
    pub fn limit_price(self) -> Option<Decimal> {
        match self {
            Self::Market => None,
            Self::Limit { limit_price } => Some(limit_price),
        }
    }
}

impl CreateRequest {
    pub fn simple(
        symbol: &str,
        qty: i32,
        side: OrderSide,
        style: SubmitOrderStyle,
    ) -> Result<Self, Error> {
        Self::simple_with_extended_hours(symbol, qty, side, style, None)
    }

    pub fn simple_with_extended_hours(
        symbol: &str,
        qty: i32,
        side: OrderSide,
        style: SubmitOrderStyle,
        extended_hours: Option<bool>,
    ) -> Result<Self, Error> {
        if qty == 0 {
            return Err(Error::InvalidRequest("qty must not be 0".to_owned()));
        }

        let request = Self {
            symbol: Some(symbol.to_owned()),
            qty: Some(Decimal::from(qty.abs())),
            notional: None,
            side: Some(side),
            r#type: Some(style.order_type()),
            time_in_force: Some(TimeInForce::Day),
            limit_price: style.limit_price(),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours,
            client_order_id: None,
            order_class: None,
            take_profit: None,
            stop_loss: None,
            legs: None,
            position_intent: None,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn mleg(
        qty: i32,
        style: SubmitOrderStyle,
        legs: Vec<OptionLegRequest>,
    ) -> Result<Self, Error> {
        if qty == 0 {
            return Err(Error::InvalidRequest("qty must not be 0".to_owned()));
        }

        let request = Self {
            symbol: None,
            qty: Some(Decimal::from(qty.abs())),
            notional: None,
            side: None,
            r#type: Some(style.order_type()),
            time_in_force: Some(TimeInForce::Day),
            limit_price: style.limit_price(),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: None,
            client_order_id: None,
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(legs),
            position_intent: None,
        };
        request.validate()?;
        Ok(request)
    }
}

impl ReplaceRequest {
    #[must_use]
    pub fn from_submit_style(style: SubmitOrderStyle) -> Self {
        Self {
            qty: None,
            time_in_force: None,
            limit_price: style.limit_price(),
            stop_price: None,
            trail: None,
            client_order_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{
        CreateRequest, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
        PositionIntent, ReplaceRequest, SubmitOrderStyle, TimeInForce,
    };

    #[test]
    fn exposes_canonical_order_enum_strings() {
        assert_eq!(OrderSide::Buy.as_str(), "buy");
        assert_eq!(OrderType::Limit.as_str(), "limit");
        assert_eq!(TimeInForce::Day.as_str(), "day");
        assert_eq!(PositionIntent::SellToClose.as_str(), "sell_to_close");
        assert_eq!(OrderClass::Mleg.as_str(), "mleg");
        assert_eq!(OrderStatus::PendingReplace.as_str(), "pending_replace");
    }

    #[test]
    fn converts_decimal_quantities_to_i32() {
        let order: Order = serde_json::from_value(serde_json::json!({
            "id": "order-1",
            "client_order_id": "client-1",
            "created_at": "2026-04-15T10:00:00Z",
            "updated_at": "2026-04-15T10:00:00Z",
            "submitted_at": "2026-04-15T10:00:00Z",
            "filled_at": null,
            "expired_at": null,
            "expires_at": null,
            "canceled_at": null,
            "failed_at": null,
            "replaced_at": null,
            "replaced_by": null,
            "replaces": null,
            "asset_id": "asset-1",
            "symbol": "AAPL",
            "asset_class": "us_equity",
            "notional": null,
            "qty": "3",
            "filled_qty": "2",
            "filled_avg_price": "187.25",
            "order_class": "simple",
            "order_type": "limit",
            "type": "limit",
            "side": "buy",
            "position_intent": null,
            "time_in_force": "day",
            "limit_price": "187.25",
            "stop_price": null,
            "status": "new",
            "extended_hours": false,
            "legs": null,
            "trail_percent": null,
            "trail_price": null,
            "hwm": null,
            "ratio_qty": null,
            "take_profit": null,
            "stop_loss": null,
            "subtag": null,
            "source": null
        }))
        .expect("order should deserialize");

        assert_eq!(order.qty_i32(), Some(3));
        assert_eq!(order.filled_qty_i32(), 2);
        assert_eq!(order.limit_price, Some(Decimal::new(18725, 2)));
    }

    #[test]
    fn builds_simple_market_request_with_day_time_in_force() {
        let request = CreateRequest::simple("SPY", 2, OrderSide::Buy, SubmitOrderStyle::Market)
            .expect("simple market request should build");

        assert_eq!(request.symbol.as_deref(), Some("SPY"));
        assert_eq!(request.qty, Some(Decimal::from(2)));
        assert_eq!(request.side, Some(OrderSide::Buy));
        assert_eq!(request.r#type, Some(OrderType::Market));
        assert_eq!(request.time_in_force, Some(TimeInForce::Day));
        assert_eq!(request.limit_price, None);
        assert_eq!(request.order_class, None);
    }

    #[test]
    fn builds_mleg_limit_request_with_validated_legs() {
        let request = CreateRequest::mleg(
            1,
            SubmitOrderStyle::Limit {
                limit_price: Decimal::new(125, 2),
            },
            vec![
                OptionLegRequest {
                    symbol: "SPY260424C00550000".to_owned(),
                    ratio_qty: 1,
                    side: Some(OrderSide::Buy),
                    position_intent: Some(PositionIntent::BuyToOpen),
                },
                OptionLegRequest {
                    symbol: "SPY260424C00555000".to_owned(),
                    ratio_qty: 1,
                    side: Some(OrderSide::Sell),
                    position_intent: Some(PositionIntent::SellToOpen),
                },
            ],
        )
        .expect("mleg limit request should build");

        assert_eq!(request.symbol, None);
        assert_eq!(request.qty, Some(Decimal::ONE));
        assert_eq!(request.side, None);
        assert_eq!(request.r#type, Some(OrderType::Limit));
        assert_eq!(request.limit_price, Some(Decimal::new(125, 2)));
        assert_eq!(request.order_class, Some(OrderClass::Mleg));
        assert_eq!(request.legs.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn builds_replace_request_from_submit_style() {
        let market = ReplaceRequest::from_submit_style(SubmitOrderStyle::Market);
        assert_eq!(market.limit_price, None);

        let limit = ReplaceRequest::from_submit_style(SubmitOrderStyle::Limit {
            limit_price: Decimal::new(333, 2),
        });
        assert_eq!(limit.limit_price, Some(Decimal::new(333, 2)));
    }
}
