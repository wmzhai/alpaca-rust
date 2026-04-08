#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::Serialize;

use alpaca_trade::orders::OrderStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityEventKind {
    New,
    Filled,
    Canceled,
    Replaced,
}

impl ActivityEventKind {
    pub(crate) fn as_activity_type(&self) -> &'static str {
        match self {
            Self::New => "NEW",
            Self::Filled => "FILL",
            Self::Canceled => "CANCELED",
            Self::Replaced => "REPLACED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ProjectedActivity {
    pub(crate) id: String,
    pub(crate) activity_type: String,
    pub(crate) transaction_time: String,
    pub(crate) order_id: String,
    pub(crate) client_order_id: String,
    pub(crate) related_order_id: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) symbol: String,
    pub(crate) asset_class: String,
    #[serde(serialize_with = "serialize_decimal_string")]
    pub(crate) net_amount: Decimal,
}

pub(crate) fn project_activity(event: &ActivityEvent) -> ProjectedActivity {
    ProjectedActivity {
        id: format!("mock-activity-{}", event.sequence),
        activity_type: event.kind.as_activity_type().to_owned(),
        transaction_time: event.occurred_at.clone(),
        order_id: event.order_id.clone(),
        client_order_id: event.client_order_id.clone(),
        related_order_id: event.related_order_id.clone(),
        status: event.status.as_ref().map(order_status_name),
        symbol: event.symbol.clone(),
        asset_class: event.asset_class.clone(),
        net_amount: event.cash_delta,
    }
}

pub(crate) fn matches_activity_type(event: &ActivityEvent, filter: &str) -> bool {
    event.kind.as_activity_type().eq_ignore_ascii_case(filter)
}

fn order_status_name(status: &OrderStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{status:?}"))
}

fn serialize_decimal_string<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}
