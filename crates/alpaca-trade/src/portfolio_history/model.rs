use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortfolioHistory {
    pub timestamp: Vec<i64>,
    #[serde(
        deserialize_with = "deserialize_decimal_vec_from_string_or_number",
        serialize_with = "serialize_decimal_vec_as_numbers"
    )]
    pub equity: Vec<Decimal>,
    #[serde(
        deserialize_with = "deserialize_decimal_vec_from_string_or_number",
        serialize_with = "serialize_decimal_vec_as_numbers"
    )]
    pub profit_loss: Vec<Decimal>,
    #[serde(
        deserialize_with = "deserialize_decimal_vec_from_string_or_number",
        serialize_with = "serialize_decimal_vec_as_numbers"
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(serde_json::Number),
}

fn parse_decimal<E>(value: StringOrNumber) -> Result<Decimal, E>
where
    E: de::Error,
{
    let raw = match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    };

    Decimal::from_str(&raw)
        .map_err(|error| E::custom(format!("invalid decimal value `{raw}`: {error}")))
}

fn deserialize_decimal_vec_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Vec<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<StringOrNumber>::deserialize(deserializer)?
        .into_iter()
        .map(parse_decimal)
        .collect()
}

fn serialize_decimal_vec_as_numbers<S>(
    values: &[Decimal],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let json_values = values
        .iter()
        .map(|value| {
            serde_json::Number::from_str(&value.to_string()).map(serde_json::Value::Number)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            serde::ser::Error::custom(format!(
                "decimal vector cannot be serialized as JSON numbers: {error}"
            ))
        })?;

    json_values.serialize(serializer)
}
