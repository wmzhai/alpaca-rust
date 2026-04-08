use rust_decimal::Decimal;

use alpaca_trade::{
    activities::Activity,
    orders::{Order, OrderSide, OrderStatus},
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityEventKind {
    New,
    Filled,
    Canceled,
    Replaced,
    PositionClosed,
    Exercised,
    DoNotExercise,
}

impl ActivityEventKind {
    pub(crate) fn public_activity_type(&self) -> Option<&'static str> {
        match self {
            Self::Filled => Some("FILL"),
            Self::New
            | Self::Canceled
            | Self::Replaced
            | Self::PositionClosed
            | Self::Exercised
            | Self::DoNotExercise => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivityEvent {
    pub(crate) sequence: u64,
    pub(crate) kind: ActivityEventKind,
    pub(crate) order_id: String,
    pub(crate) client_order_id: String,
    pub(crate) related_order_id: Option<String>,
    pub(crate) status: Option<OrderStatus>,
    pub(crate) symbol: String,
    pub(crate) asset_class: String,
    pub(crate) occurred_at: String,
    pub(crate) cash_delta: Decimal,
    pub(crate) r#type: Option<String>,
    pub(crate) price: Option<Decimal>,
    pub(crate) qty: Option<Decimal>,
    pub(crate) side: Option<OrderSide>,
    pub(crate) leaves_qty: Option<Decimal>,
    pub(crate) cum_qty: Option<Decimal>,
}

impl ActivityEvent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        sequence: u64,
        kind: ActivityEventKind,
        order_id: String,
        client_order_id: String,
        related_order_id: Option<String>,
        status: Option<OrderStatus>,
        symbol: String,
        asset_class: String,
        occurred_at: String,
        cash_delta: Decimal,
    ) -> Self {
        Self {
            sequence,
            kind,
            order_id,
            client_order_id,
            related_order_id,
            status,
            symbol,
            asset_class,
            occurred_at,
            cash_delta,
            r#type: None,
            price: None,
            qty: None,
            side: None,
            leaves_qty: None,
            cum_qty: None,
        }
    }

    pub(crate) fn with_fill_order(mut self, order: &Order, side: &OrderSide) -> Self {
        self.r#type = Some("fill".to_owned());
        self.price = order.filled_avg_price;
        self.qty = Some(order.filled_qty);
        self.side = Some(side.clone());
        self.leaves_qty = Some(Decimal::ZERO);
        self.cum_qty = Some(order.filled_qty);
        self
    }
}

pub(crate) fn project_activity(event: &ActivityEvent) -> Option<Activity> {
    Some(Activity {
        id: format!("mock-activity-{}", event.sequence),
        activity_type: event.kind.public_activity_type()?.to_owned(),
        transaction_time: Some(event.occurred_at.clone()),
        r#type: event.r#type.clone(),
        price: event.price,
        qty: event.qty,
        side: event.side.as_ref().map(order_side_name),
        symbol: Some(event.symbol.clone()),
        leaves_qty: event.leaves_qty,
        order_id: Some(event.order_id.clone()),
        cum_qty: event.cum_qty,
        order_status: event.status.as_ref().map(order_status_name),
        extra: Default::default(),
    })
}

pub(crate) fn matches_activity_type(event: &ActivityEvent, filter: &str) -> bool {
    event
        .kind
        .public_activity_type()
        .is_some_and(|activity_type| activity_type.eq_ignore_ascii_case(filter))
}

pub(crate) fn is_public_activity(event: &ActivityEvent) -> bool {
    event.kind.public_activity_type().is_some()
}

fn order_side_name(side: &OrderSide) -> String {
    serde_json::to_value(side)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{side:?}"))
}

fn order_status_name(status: &OrderStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{status:?}"))
}
