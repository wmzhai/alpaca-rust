use rust_decimal::prelude::ToPrimitive;

use super::{Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent, TimeInForce};

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
        matches!(self, Self::Canceled | Self::Filled | Self::Expired | Self::Rejected)
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

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{
        Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent, TimeInForce,
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
}
