use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

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

fn round(value: &Decimal, scale: u32) -> Decimal {
    value.round_dp(scale)
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

    pub fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        string_contract::serialize_decimal(&round(value, PRICE_SCALE), serializer)
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
}
