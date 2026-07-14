use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::assets::AssetClass;
use crate::orders::Order;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub asset_id: String,
    pub symbol: String,
    pub exchange: PositionExchange,
    pub asset_class: AssetClass,
    pub asset_marginable: bool,
    pub side: PositionSide,
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
    pub unrealized_intraday_pl: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub unrealized_intraday_plpc: Decimal,
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
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub qty_available: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub avg_entry_swap_rate: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub prev_swap_rate: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub swap_rate: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd: Option<UsdPositionValues>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionExchange {
    #[serde(rename = "AMEX")]
    Amex,
    #[serde(rename = "ARCA")]
    Arca,
    #[serde(rename = "BATS")]
    Bats,
    #[serde(rename = "NYSE")]
    Nyse,
    #[serde(rename = "NASDAQ")]
    Nasdaq,
    #[serde(rename = "NYSEARCA")]
    NyseArca,
    #[serde(rename = "OTC")]
    Otc,
    #[serde(rename = "CRYPTO")]
    Crypto,
    #[default]
    #[serde(rename = "")]
    Unspecified,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    #[default]
    Long,
    Short,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UsdPositionValues {
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub avg_entry_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub cost_basis: Decimal,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub market_value: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub unrealized_pl: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub unrealized_plpc: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub unrealized_intraday_pl: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub unrealized_intraday_plpc: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub current_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub lastday_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal",
        skip_serializing_if = "Option::is_none"
    )]
    pub change_today: Option<Decimal>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClosePositionResult {
    pub symbol: String,
    pub status: u16,
    pub body: Option<Order>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExerciseAccepted {
    pub details: Option<ExerciseDetails>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExerciseDetails {
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
