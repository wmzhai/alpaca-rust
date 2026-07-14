use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::Error;

use super::{
    CreateRequest, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    OrdersClient, PositionIntent, ReplaceRequest, ResolvedOrder, TimeInForce, WaitFor,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubmitOrderStyle {
    Market,
    Limit { limit_price: Decimal },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitOrderPolicy {
    Default,
    AcceptOnly,
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
        position_intent: Option<PositionIntent>,
        client_order_id: Option<String>,
    },
    Mleg {
        qty: i32,
        style: SubmitOrderStyle,
        legs: Vec<OptionLegRequest>,
        client_order_id: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionOrderPolicy {
    Auto,
    Recreate,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransitionResolution {
    NewOrder {
        resolved: ResolvedOrder,
        recreated: bool,
    },
    OriginalOrderTerminal(ResolvedOrder),
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
pub enum CloseOptionLegsResult {
    Filled {
        order: Order,
        legs: Vec<ClosedOptionLeg>,
        cashflow: Decimal,
    },
    Submitted {
        order: Order,
        legs: Vec<ClosedOptionLeg>,
    },
    Skipped {
        legs: Vec<ClosedOptionLeg>,
    },
}

impl CloseOptionLegsResult {
    #[must_use]
    pub fn legs(&self) -> &[ClosedOptionLeg] {
        match self {
            Self::Filled { legs, .. } | Self::Submitted { legs, .. } | Self::Skipped { legs } => {
                legs
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarketCloseRecovery {
    RetryAsMarket {
        next_retry_count: i32,
    },
    Fallback {
        next_retry_count: i32,
        result: CloseOptionLegsResult,
    },
}

impl MarketCloseRecovery {
    #[must_use]
    pub fn next_retry_count(&self) -> i32 {
        match self {
            Self::RetryAsMarket { next_retry_count }
            | Self::Fallback {
                next_retry_count, ..
            } => *next_retry_count,
        }
    }
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
            "" => Ok(Self::Unspecified),
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
            Self::Unspecified => "",
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
            "failed" => Ok(Self::Failed),
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
            "" => Ok(Self::Unspecified),
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
            Self::Failed => "failed",
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
            Self::Unspecified => "",
        }
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled
                | Self::Failed
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
        matches!(
            self,
            Self::Failed | Self::Canceled | Self::Expired | Self::Rejected
        )
    }

    #[must_use]
    pub fn is_stable(self) -> bool {
        matches!(
            self,
            Self::Accepted
                | Self::New
                | Self::Filled
                | Self::Failed
                | Self::Canceled
                | Self::Expired
                | Self::Rejected
        )
    }

    #[must_use]
    pub fn is_cancel_complete(self) -> bool {
        matches!(
            self,
            Self::Canceled | Self::Filled | Self::Failed | Self::Expired | Self::Rejected
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

    fn has_fill_evidence(&self) -> bool {
        self.filled_qty > Decimal::ZERO
            || self
                .legs
                .as_deref()
                .is_some_and(|legs| legs.iter().any(|leg| leg.filled_qty > Decimal::ZERO))
    }

    fn can_recreate_after_cancel(&self, policy: TransitionOrderPolicy) -> bool {
        self.status == OrderStatus::Canceled
            && (!matches!(policy, TransitionOrderPolicy::Recreate) || !self.has_fill_evidence())
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
            position_intent: None,
            client_order_id: None,
        }
    }

    #[must_use]
    pub fn mleg(qty: i32, style: SubmitOrderStyle, legs: Vec<OptionLegRequest>) -> Self {
        Self::Mleg {
            qty,
            style,
            legs,
            client_order_id: None,
        }
    }

    #[must_use]
    pub fn with_client_order_id(mut self, client_order_id: impl Into<String>) -> Self {
        match &mut self {
            Self::Simple {
                client_order_id: current,
                ..
            }
            | Self::Mleg {
                client_order_id: current,
                ..
            } => *current = Some(client_order_id.into()),
        }
        self
    }

    #[must_use]
    pub fn with_position_intent(mut self, position_intent: PositionIntent) -> Self {
        if let Self::Simple {
            position_intent: current,
            ..
        } = &mut self
        {
            *current = Some(position_intent);
        }
        self
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

    #[must_use]
    pub fn wait_for(self, policy: SubmitOrderPolicy) -> WaitFor {
        match policy {
            SubmitOrderPolicy::Default => self.default_wait_for(),
            SubmitOrderPolicy::AcceptOnly => WaitFor::Stable,
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
                position_intent,
                client_order_id,
            } => {
                let mut request = CreateRequest::simple(
                    &symbol,
                    qty,
                    side,
                    style,
                    time_in_force,
                    extended_hours,
                )?;
                request.position_intent = position_intent;
                request.client_order_id = client_order_id;
                request.validate()?;
                Ok(request)
            }
            Self::Mleg {
                qty,
                style,
                legs,
                client_order_id,
            } => {
                let mut request = CreateRequest::mleg(qty, style, legs)?;
                request.client_order_id = client_order_id;
                request.validate()?;
                Ok(request)
            }
        }
    }

    fn requires_recreate(&self, current_order: &Order, policy: TransitionOrderPolicy) -> bool {
        if matches!(policy, TransitionOrderPolicy::Recreate) {
            return true;
        }

        if self.has_close_mleg_legs() {
            return true;
        }

        !self.is_replace_compatible(current_order)
    }

    fn is_replace_compatible(&self, current_order: &Order) -> bool {
        let same_style = current_order.r#type == self.style().order_type();
        match self {
            Self::Simple { .. } => current_order.order_class != OrderClass::Mleg && same_style,
            Self::Mleg { .. } => current_order.order_class == OrderClass::Mleg && same_style,
        }
    }

    fn has_close_mleg_legs(&self) -> bool {
        match self {
            Self::Simple { .. } => false,
            Self::Mleg { legs, .. } => legs.iter().any(|leg| {
                leg.position_intent.is_some_and(|intent| {
                    intent == PositionIntent::BuyToClose || intent == PositionIntent::SellToClose
                })
            }),
        }
    }
}

impl TransitionResolution {
    #[must_use]
    pub fn order(&self) -> &Order {
        match self {
            Self::NewOrder { resolved, .. } | Self::OriginalOrderTerminal(resolved) => {
                &resolved.order
            }
        }
    }

    #[must_use]
    pub fn recovered_after_request_error(&self) -> bool {
        match self {
            Self::NewOrder { resolved, .. } | Self::OriginalOrderTerminal(resolved) => {
                resolved.recovered_after_request_error
            }
        }
    }

    #[must_use]
    pub fn recreated(&self) -> bool {
        matches!(
            self,
            Self::NewOrder {
                recreated: true,
                ..
            }
        )
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
            advanced_instructions: None,
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
            advanced_instructions: None,
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
            advanced_instructions: None,
        }
    }
}

impl OrdersClient {
    pub async fn submit_with_policy(
        &self,
        request: SubmitOrderRequest,
        policy: SubmitOrderPolicy,
    ) -> Result<crate::orders::ResolvedOrder, Error> {
        let target = request.clone().wait_for(policy);
        self.create_resolved(request.into_create_request()?, target)
            .await
    }

    pub async fn submit_resolved(
        &self,
        request: SubmitOrderRequest,
        wait_for: Option<WaitFor>,
    ) -> Result<crate::orders::ResolvedOrder, Error> {
        let target = wait_for.unwrap_or_else(|| request.default_wait_for());
        self.create_resolved(request.into_create_request()?, target)
            .await
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
            return Ok(CloseOptionLegsResult::Skipped { legs: closed_legs });
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
            return Ok(CloseOptionLegsResult::Submitted {
                order,
                legs: closed_legs,
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

        Ok(CloseOptionLegsResult::Filled {
            order,
            legs: closed_legs,
            cashflow,
        })
    }

    pub async fn recover_market_close(
        &self,
        retry_count: i32,
        max_retries: i32,
        qty: i32,
        legs: Vec<CloseOptionLeg>,
    ) -> Result<MarketCloseRecovery, Error> {
        let next_retry_count = retry_count.max(0).saturating_add(1);
        if next_retry_count <= max_retries.max(0) {
            return Ok(MarketCloseRecovery::RetryAsMarket { next_retry_count });
        }

        let result = self.close_option_legs(qty, legs).await?;
        Ok(MarketCloseRecovery::Fallback {
            next_retry_count,
            result,
        })
    }

    pub async fn transition_resolved(
        &self,
        order_id: &str,
        request: SubmitOrderRequest,
        policy: TransitionOrderPolicy,
    ) -> Result<TransitionResolution, Error> {
        let current_order = self.get_effective(order_id).await?;
        if request.requires_recreate(&current_order, policy) {
            let wait_for = request.default_wait_for();
            let create_request = request.clone().into_create_request()?;
            if matches!(policy, TransitionOrderPolicy::Recreate) {
                if create_request.client_order_id.is_none() {
                    return Err(Error::InvalidRequest(
                        "recreate transition requires client_order_id".to_owned(),
                    ));
                }
                if let Some(recovered) =
                    self.recover_created_once(&create_request, wait_for).await?
                {
                    return Ok(TransitionResolution::NewOrder {
                        resolved: recovered,
                        recreated: true,
                    });
                }
            }

            let canceled = self.cancel_resolved(&current_order.id).await?;
            if !canceled.order.can_recreate_after_cancel(policy) {
                return Ok(TransitionResolution::OriginalOrderTerminal(canceled));
            }

            let recreated = self.create_resolved(create_request, wait_for).await?;
            return Ok(TransitionResolution::NewOrder {
                resolved: ResolvedOrder {
                    order: recreated.order,
                    recovered_after_request_error: canceled.recovered_after_request_error
                        || recreated.recovered_after_request_error,
                },
                recreated: true,
            });
        }

        match self
            .replace_resolved(
                &current_order.id,
                ReplaceRequest::from_submit_style(request.style()),
            )
            .await?
        {
            super::ReplaceResolution::NewOrder(resolved) => Ok(TransitionResolution::NewOrder {
                resolved,
                recreated: false,
            }),
            super::ReplaceResolution::OriginalOrderTerminal(resolved) => {
                Ok(TransitionResolution::OriginalOrderTerminal(resolved))
            }
        }
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
