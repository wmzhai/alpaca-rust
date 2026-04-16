use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortfolioHistory {
    pub timestamp: Vec<i64>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_vec_from_string_or_number",
        serialize_with = "alpaca_core::decimal::serialize_decimal_vec_as_numbers"
    )]
    pub equity: Vec<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_vec_from_string_or_number",
        serialize_with = "alpaca_core::decimal::serialize_decimal_vec_as_numbers"
    )]
    pub profit_loss: Vec<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_vec_from_string_or_number",
        serialize_with = "alpaca_core::decimal::serialize_decimal_vec_as_numbers"
    )]
    pub profit_loss_pct: Vec<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub base_value: Decimal,
    pub base_value_asof: Option<String>,
    pub timeframe: String,
    pub cashflow: Option<serde_json::Value>,
}
