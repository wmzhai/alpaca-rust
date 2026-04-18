use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;

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

fn rounded(value: &Decimal, scale: u32) -> Decimal {
    value.round_dp(scale)
}

pub fn from_f64(value: f64, scale: u32) -> Decimal {
    if !value.is_finite() {
        return Decimal::ZERO;
    }

    Decimal::from_f64_retain(value)
        .unwrap_or_default()
        .round_dp(scale)
}

pub fn round(value: Decimal, scale: u32) -> Decimal {
    value.round_dp(scale)
}

pub fn format(value: Decimal, scale: u32) -> String {
    round(value, scale).to_string()
}

pub fn deserialize_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    parse_decimal(StringOrNumber::deserialize(deserializer)?)
}

pub fn deserialize_option_decimal_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrNumber>::deserialize(deserializer)?
        .map(parse_decimal)
        .transpose()
}

pub fn deserialize_scaled_decimal_from_string_or_number<'de, D>(
    deserializer: D,
    scale: u32,
) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_decimal_from_string_or_number(deserializer).map(|value| round(value, scale))
}

pub fn deserialize_decimal_vec_from_string_or_number<'de, D>(
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

pub fn serialize_decimal_vec_as_numbers<S>(
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

pub fn parse_json_decimal(value: Option<&Value>) -> Option<Decimal> {
    value.and_then(|value| match value {
        Value::String(raw) => Decimal::from_str(raw).ok(),
        Value::Number(raw) => Decimal::from_str(&raw.to_string()).ok(),
        _ => None,
    })
}

pub fn parse_json_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| match value {
        Value::String(raw) => raw.parse::<f64>().ok().filter(|number| number.is_finite()),
        Value::Number(raw) => raw.as_f64().filter(|number| number.is_finite()),
        _ => None,
    })
}

pub mod string_contract {
    use super::*;

    pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn serialize_option_decimal<S>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_decimal(value, serializer),
            None => serializer.serialize_none(),
        }
    }
}

pub mod price_string_contract {
    use super::*;

    const PRICE_SCALE: u32 = 2;

    pub fn serialize<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_decimal(value, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_scaled_decimal_from_string_or_number(deserializer, PRICE_SCALE)
    }

    pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        string_contract::serialize_decimal(&rounded(value, PRICE_SCALE), serializer)
    }

    pub fn serialize_option_decimal<S>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_decimal(value, serializer),
            None => serializer.serialize_none(),
        }
    }
}

pub mod number_contract {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_decimal_from_string_or_number(deserializer)
    }

    pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_json::Value::Number(decimal_to_json_number::<S>(value)?).serialize(serializer)
    }

    pub fn serialize_option_decimal<S>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_decimal(value, serializer),
            None => serializer.serialize_none(),
        }
    }

    fn decimal_to_json_number<S>(value: &Decimal) -> Result<serde_json::Number, S::Error>
    where
        S: Serializer,
    {
        serde_json::Number::from_str(&value.to_string()).map_err(|error| {
            serde::ser::Error::custom(format!(
                "decimal cannot be serialized as JSON number: {error}"
            ))
        })
    }

    pub mod option_decimal {
        use super::*;

        pub fn serialize<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            super::serialize_option_decimal(value, serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_option_decimal_from_string_or_number(deserializer)
        }
    }
}
