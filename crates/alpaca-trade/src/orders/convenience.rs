use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::Error;

use super::{
    CreateRequest, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    OrdersClient, PositionIntent, ReplaceRequest, TimeInForce, WaitFor,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubmitOrderStyle {
    Market,
    Limit { limit_price: Decimal },
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubmitOrderRequest {
    Simple {
        symbol: String,
        qty: i32,
        side: OrderSide,
        style: SubmitOrderStyle,
        time_in_force: Option<TimeInForce>,
        extended_hours: Option<bool>,
    },
    Mleg {
        qty: i32,
        style: SubmitOrderStyle,
        legs: Vec<OptionLegRequest>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionQuote {
    pub bid: Decimal,
    pub ask: Decimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloseOptionLeg {
    pub symbol: String,
    pub ratio_qty: u32,
    pub side: OrderSide,
    pub position_intent: PositionIntent,
    pub quote: Option<OptionQuote>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClosedOptionLeg {
    pub symbol: String,
    pub ratio_qty: u32,
    pub side: OrderSide,
    pub position_intent: PositionIntent,
    pub filled_avg_price: Decimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloseOptionLegsResult {
    pub status: CloseOptionLegsStatus,
    pub order: Option<Order>,
    pub legs: Vec<ClosedOptionLeg>,
    pub cashflow: Decimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseOptionLegsStatus {
    Filled,
    Submitted,
    Skipped,
}

impl OrderSide {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value.trim() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            _ => Err(Error::InvalidRequest(format!(
                "invalid order side: {}",
                value
            ))),
        }
    }

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
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value.trim() {
            "day" => Ok(Self::Day),
            "gtc" => Ok(Self::Gtc),
            "opg" => Ok(Self::Opg),
            "cls" => Ok(Self::Cls),
            "ioc" => Ok(Self::Ioc),
            "fok" => Ok(Self::Fok),
            "gtd" => Ok(Self::Gtd),
            _ => Err(Error::InvalidRequest(format!(
                "invalid time in force: {}",
                value
            ))),
        }
    }

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
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value.trim() {
            "buy_to_open" => Ok(Self::BuyToOpen),
            "buy_to_close" => Ok(Self::BuyToClose),
            "sell_to_open" => Ok(Self::SellToOpen),
            "sell_to_close" => Ok(Self::SellToClose),
            _ => Err(Error::InvalidRequest(format!(
                "invalid position intent: {}",
                value
            ))),
        }
    }

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
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value.trim() {
            "new" => Ok(Self::New),
            "partially_filled" => Ok(Self::PartiallyFilled),
            "filled" => Ok(Self::Filled),
            "done_for_day" => Ok(Self::DoneForDay),
            "canceled" => Ok(Self::Canceled),
            "expired" => Ok(Self::Expired),
            "replaced" => Ok(Self::Replaced),
            "pending_cancel" => Ok(Self::PendingCancel),
            "pending_replace" => Ok(Self::PendingReplace),
            "accepted" => Ok(Self::Accepted),
            "pending_new" => Ok(Self::PendingNew),
            "accepted_for_bidding" => Ok(Self::AcceptedForBidding),
            "stopped" => Ok(Self::Stopped),
            "rejected" => Ok(Self::Rejected),
            "suspended" => Ok(Self::Suspended),
            "calculated" => Ok(Self::Calculated),
            "held" => Ok(Self::Held),
            _ => Err(Error::InvalidRequest(format!(
                "invalid order status: {}",
                value
            ))),
        }
    }

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
    pub fn is_filled(self) -> bool {
        matches!(self, Self::Filled)
    }

    #[must_use]
    pub fn is_failed_terminal(self) -> bool {
        matches!(self, Self::Canceled | Self::Expired | Self::Rejected)
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

impl CloseOptionLeg {
    #[must_use]
    pub fn is_liquid(&self) -> bool {
        let Some(quote) = &self.quote else {
            return false;
        };

        match self.side {
            OrderSide::Sell => quote.bid > Decimal::ZERO,
            OrderSide::Buy => quote.ask > Decimal::ZERO,
            OrderSide::Unspecified => false,
        }
    }

    fn contract_qty(&self, structure_qty: i32) -> Result<i32, Error> {
        if structure_qty <= 0 {
            return Err(Error::InvalidRequest(
                "structure qty must be greater than 0".to_owned(),
            ));
        }
        if self.ratio_qty == 0 {
            return Err(Error::InvalidRequest(
                "ratio_qty must be greater than 0".to_owned(),
            ));
        }

        structure_qty
            .checked_mul(i32::try_from(self.ratio_qty).map_err(|_| {
                Error::InvalidRequest("ratio_qty exceeds supported range".to_owned())
            })?)
            .ok_or_else(|| Error::InvalidRequest("contract qty exceeds supported range".to_owned()))
    }

    fn to_option_leg_request(&self) -> Result<OptionLegRequest, Error> {
        if self.ratio_qty == 0 {
            return Err(Error::InvalidRequest(
                "ratio_qty must be greater than 0".to_owned(),
            ));
        }

        Ok(OptionLegRequest {
            symbol: self.symbol.clone(),
            ratio_qty: self.ratio_qty,
            side: Some(self.side),
            position_intent: Some(self.position_intent),
        })
    }
}

impl ClosedOptionLeg {
    fn zeroed(leg: &CloseOptionLeg) -> Self {
        Self {
            symbol: leg.symbol.clone(),
            ratio_qty: leg.ratio_qty,
            side: leg.side,
            position_intent: leg.position_intent,
            filled_avg_price: Decimal::ZERO,
        }
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

impl SubmitOrderRequest {
    #[must_use]
    pub fn simple(
        symbol: &str,
        qty: i32,
        side: OrderSide,
        style: SubmitOrderStyle,
        time_in_force: Option<TimeInForce>,
        extended_hours: Option<bool>,
    ) -> Self {
        Self::Simple {
            symbol: symbol.to_owned(),
            qty,
            side,
            style,
            time_in_force,
            extended_hours,
        }
    }

    #[must_use]
    pub fn mleg(qty: i32, style: SubmitOrderStyle, legs: Vec<OptionLegRequest>) -> Self {
        Self::Mleg { qty, style, legs }
    }

    #[must_use]
    pub fn default_wait_for(&self) -> WaitFor {
        match self.style() {
            SubmitOrderStyle::Market => WaitFor::Filled,
            SubmitOrderStyle::Limit { .. } => WaitFor::Stable,
        }
    }

    #[must_use]
    pub fn style(&self) -> SubmitOrderStyle {
        match self {
            Self::Simple { style, .. } | Self::Mleg { style, .. } => *style,
        }
    }

    pub fn into_create_request(self) -> Result<CreateRequest, Error> {
        match self {
            Self::Simple {
                symbol,
                qty,
                side,
                style,
                time_in_force,
                extended_hours,
            } => CreateRequest::simple(
                &symbol,
                qty,
                side,
                style,
                time_in_force,
                extended_hours,
            ),
            Self::Mleg { qty, style, legs } => CreateRequest::mleg(qty, style, legs),
        }
    }
}

impl CreateRequest {
    pub fn simple(
        symbol: &str,
        qty: i32,
        side: OrderSide,
        style: SubmitOrderStyle,
        time_in_force: Option<TimeInForce>,
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
            time_in_force: Some(time_in_force.unwrap_or(TimeInForce::Day)),
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

impl OrdersClient {
    pub async fn submit_resolved(
        &self,
        request: SubmitOrderRequest,
        wait_for: Option<WaitFor>,
    ) -> Result<crate::orders::ResolvedOrder, Error> {
        let target = wait_for.unwrap_or_else(|| request.default_wait_for());
        self.create_resolved(request.into_create_request()?, target).await
    }

    pub async fn close_option_legs(
        &self,
        qty: i32,
        legs: Vec<CloseOptionLeg>,
    ) -> Result<CloseOptionLegsResult, Error> {
        if qty <= 0 {
            return Err(Error::InvalidRequest(
                "structure qty must be greater than 0".to_owned(),
            ));
        }
        if legs.is_empty() {
            return Err(Error::InvalidRequest(
                "close_option_legs requires at least one leg".to_owned(),
            ));
        }

        let mut closed_legs = legs.iter().map(ClosedOptionLeg::zeroed).collect::<Vec<_>>();
        let liquid_indices = legs
            .iter()
            .enumerate()
            .filter_map(|(index, leg)| leg.is_liquid().then_some(index))
            .collect::<Vec<_>>();

        if liquid_indices.is_empty() {
            return Ok(CloseOptionLegsResult {
                status: CloseOptionLegsStatus::Skipped,
                order: None,
                legs: closed_legs,
                cashflow: Decimal::ZERO,
            });
        }

        let order = if liquid_indices.len() == 1 {
            let leg = &legs[liquid_indices[0]];
            self.create_resolved(
                CreateRequest::simple(
                    &leg.symbol,
                    leg.contract_qty(qty)?,
                    leg.side,
                    SubmitOrderStyle::Market,
                    None,
                    None,
                )?,
                WaitFor::Filled,
            )
            .await?
            .order
        } else {
            self.create_resolved(
                CreateRequest::mleg(
                    qty,
                    SubmitOrderStyle::Market,
                    liquid_indices
                        .iter()
                        .map(|&index| legs[index].to_option_leg_request())
                        .collect::<Result<Vec<_>, Error>>()?,
                )?,
                WaitFor::Filled,
            )
            .await?
            .order
        };

        if order.status != OrderStatus::Filled {
            return Ok(CloseOptionLegsResult {
                status: CloseOptionLegsStatus::Submitted,
                order: Some(order),
                legs: closed_legs,
                cashflow: Decimal::ZERO,
            });
        }

        if liquid_indices.len() == 1 {
            if let Some(price) = order.filled_avg_price {
                closed_legs[liquid_indices[0]].filled_avg_price = price;
            }
        } else if let Some(order_legs) = order.legs.as_ref() {
            for (submitted_index, &original_index) in liquid_indices.iter().enumerate() {
                if let Some(price) = order_legs
                    .get(submitted_index)
                    .and_then(|leg| leg.filled_avg_price)
                {
                    closed_legs[original_index].filled_avg_price = price;
                }
            }
        }

        let mut cashflow = closed_legs_cashflow(&closed_legs, qty);
        if cashflow.is_zero() && liquid_indices.len() > 1 {
            if let Some(price) = order.filled_avg_price {
                cashflow = (-price * Decimal::from(qty) * Decimal::from(100)).round_dp(2);
            }
        }

        Ok(CloseOptionLegsResult {
            status: CloseOptionLegsStatus::Filled,
            order: Some(order),
            legs: closed_legs,
            cashflow,
        })
    }
}

fn closed_legs_cashflow(legs: &[ClosedOptionLeg], structure_qty: i32) -> Decimal {
    legs.iter()
        .map(|leg| leg_cashflow(leg.filled_avg_price, leg.side, structure_qty, leg.ratio_qty))
        .fold(Decimal::ZERO, |total, value| total + value)
        .round_dp(2)
}

fn leg_cashflow(price: Decimal, side: OrderSide, structure_qty: i32, ratio_qty: u32) -> Decimal {
    if price.is_zero() || structure_qty <= 0 || ratio_qty == 0 {
        return Decimal::ZERO;
    }

    let contracts = Decimal::from(i64::from(structure_qty) * i64::from(ratio_qty));
    let gross = match side {
        OrderSide::Sell => price * contracts * Decimal::from(100),
        OrderSide::Buy => -price * contracts * Decimal::from(100),
        OrderSide::Unspecified => Decimal::ZERO,
    };

    gross.round_dp(2)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{
        CloseOptionLeg, CreateRequest, OptionLegRequest, OptionQuote, Order, OrderClass, OrderSide,
        OrderStatus, OrderType, PositionIntent, ReplaceRequest, SubmitOrderRequest,
        SubmitOrderStyle, TimeInForce, WaitFor,
    };

    #[test]
    fn exposes_canonical_order_enum_strings() {
        assert_eq!(OrderSide::Buy.as_str(), "buy");
        assert_eq!(OrderType::Limit.as_str(), "limit");
        assert_eq!(TimeInForce::Day.as_str(), "day");
        assert_eq!(PositionIntent::SellToClose.as_str(), "sell_to_close");
        assert_eq!(OrderClass::Mleg.as_str(), "mleg");
        assert_eq!(OrderStatus::PendingReplace.as_str(), "pending_replace");
        assert_eq!(
            OrderStatus::parse("rejected").expect("status should parse"),
            OrderStatus::Rejected
        );
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
        let request = CreateRequest::simple(
            "SPY",
            2,
            OrderSide::parse("buy").expect("buy should parse"),
            SubmitOrderStyle::Market,
            Some(TimeInForce::Cls),
            Some(true),
        )
        .expect("simple market request should build");

        assert_eq!(request.symbol.as_deref(), Some("SPY"));
        assert_eq!(request.qty, Some(Decimal::from(2)));
        assert_eq!(request.side, Some(OrderSide::Buy));
        assert_eq!(request.r#type, Some(OrderType::Market));
        assert_eq!(request.time_in_force, Some(TimeInForce::Cls));
        assert_eq!(request.limit_price, None);
        assert_eq!(request.extended_hours, Some(true));
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

    #[test]
    fn submit_request_defaults_wait_target_from_style() {
        let market = SubmitOrderRequest::simple(
            "SPY",
            1,
            OrderSide::Buy,
            SubmitOrderStyle::Market,
            None,
            None,
        );
        assert_eq!(market.default_wait_for(), WaitFor::Filled);

        let limit = SubmitOrderRequest::mleg(
            1,
            SubmitOrderStyle::Limit {
                limit_price: Decimal::new(125, 2),
            },
            vec![OptionLegRequest {
                symbol: "SPY260424C00550000".to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Buy),
                position_intent: Some(PositionIntent::BuyToOpen),
            }],
        );
        assert_eq!(limit.default_wait_for(), WaitFor::Stable);
    }

    #[test]
    fn submit_request_converts_into_create_request() {
        let simple = SubmitOrderRequest::simple(
            "SPY",
            2,
            OrderSide::Buy,
            SubmitOrderStyle::Limit {
                limit_price: Decimal::new(321, 2),
            },
            Some(TimeInForce::Day),
            Some(true),
        )
        .into_create_request()
        .expect("simple submit request should build");
        assert_eq!(simple.symbol.as_deref(), Some("SPY"));
        assert_eq!(simple.limit_price, Some(Decimal::new(321, 2)));
        assert_eq!(simple.extended_hours, Some(true));

        let mleg = SubmitOrderRequest::mleg(
            2,
            SubmitOrderStyle::Market,
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
        .into_create_request()
        .expect("mleg submit request should build");
        assert_eq!(mleg.order_class, Some(OrderClass::Mleg));
        assert_eq!(mleg.qty, Some(Decimal::from(2)));
        assert_eq!(mleg.r#type, Some(OrderType::Market));
    }

    #[test]
    fn detects_option_leg_liquidity_from_side_specific_quote() {
        let sell_close = CloseOptionLeg {
            symbol: "SPY260424C00550000".to_owned(),
            ratio_qty: 1,
            side: OrderSide::Sell,
            position_intent: PositionIntent::SellToClose,
            quote: Some(OptionQuote {
                bid: Decimal::new(15, 2),
                ask: Decimal::new(20, 2),
            }),
        };
        assert!(sell_close.is_liquid());

        let buy_close = CloseOptionLeg {
            symbol: "SPY260424C00555000".to_owned(),
            ratio_qty: 1,
            side: OrderSide::Buy,
            position_intent: PositionIntent::BuyToClose,
            quote: Some(OptionQuote {
                bid: Decimal::ZERO,
                ask: Decimal::new(10, 2),
            }),
        };
        assert!(buy_close.is_liquid());
    }
}
