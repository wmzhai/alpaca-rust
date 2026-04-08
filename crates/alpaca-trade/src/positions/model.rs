use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::orders::{
    Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent, StopLoss, TakeProfit,
    TimeInForce,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub asset_id: String,
    pub symbol: String,
    pub exchange: String,
    pub asset_class: String,
    pub asset_marginable: bool,
    pub side: String,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub qty: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub avg_entry_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub market_value: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub cost_basis: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub unrealized_pl: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub unrealized_plpc: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub current_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub lastday_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub change_today: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub qty_available: Decimal,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClosePositionResult {
    pub symbol: String,
    pub status: u16,
    pub body: Option<ClosePositionBody>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClosePositionBody {
    pub id: String,
    pub client_order_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub submitted_at: String,
    pub filled_at: Option<String>,
    pub expired_at: Option<String>,
    pub expires_at: Option<String>,
    pub canceled_at: Option<String>,
    pub failed_at: Option<String>,
    pub replaced_at: Option<String>,
    pub replaced_by: Option<String>,
    pub replaces: Option<String>,
    pub asset_id: String,
    pub symbol: String,
    pub asset_class: String,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub notional: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub qty: Option<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub filled_qty: Decimal,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub filled_avg_price: Option<Decimal>,
    pub order_class: OrderClass,
    pub order_type: OrderType,
    #[serde(rename = "type")]
    pub r#type: OrderType,
    pub side: OrderSide,
    pub position_intent: Option<PositionIntent>,
    pub time_in_force: TimeInForce,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub limit_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub stop_price: Option<Decimal>,
    pub status: OrderStatus,
    pub extended_hours: bool,
    pub legs: Option<Vec<ClosePositionBody>>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub trail_percent: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub trail_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub hwm: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::integer::deserialize_option_u32_from_string_or_number",
        serialize_with = "alpaca_core::integer::string_contract::serialize_option_u32"
    )]
    pub ratio_qty: Option<u32>,
    pub take_profit: Option<TakeProfit>,
    pub stop_loss: Option<StopLoss>,
    pub subtag: Option<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExercisePositionBody {
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub qty_exercised: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub qty_remaining: Decimal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoNotExerciseAccepted;

impl From<Order> for ClosePositionBody {
    fn from(order: Order) -> Self {
        Self {
            id: order.id,
            client_order_id: order.client_order_id,
            created_at: order.created_at,
            updated_at: order.updated_at,
            submitted_at: order.submitted_at,
            filled_at: order.filled_at,
            expired_at: order.expired_at,
            expires_at: order.expires_at,
            canceled_at: order.canceled_at,
            failed_at: order.failed_at,
            replaced_at: order.replaced_at,
            replaced_by: order.replaced_by,
            replaces: order.replaces,
            asset_id: order.asset_id,
            symbol: order.symbol,
            asset_class: order.asset_class,
            notional: order.notional,
            qty: order.qty,
            filled_qty: order.filled_qty,
            filled_avg_price: order.filled_avg_price,
            order_class: order.order_class,
            order_type: order.order_type,
            r#type: order.r#type,
            side: order.side,
            position_intent: order.position_intent,
            time_in_force: order.time_in_force,
            limit_price: order.limit_price,
            stop_price: order.stop_price,
            status: order.status,
            extended_hours: order.extended_hours,
            legs: order
                .legs
                .map(|legs| legs.into_iter().map(ClosePositionBody::from).collect()),
            trail_percent: order.trail_percent,
            trail_price: order.trail_price,
            hwm: order.hwm,
            ratio_qty: order.ratio_qty,
            take_profit: order.take_profit,
            stop_loss: order.stop_loss,
            subtag: order.subtag,
            source: order.source,
        }
    }
}
