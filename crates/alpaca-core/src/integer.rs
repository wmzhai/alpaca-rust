use serde::{Deserialize, Deserializer, Serializer, de};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(serde_json::Number),
}

fn parse_u32<E>(value: StringOrNumber) -> Result<u32, E>
where
    E: de::Error,
{
    let raw = match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    };

    raw.parse::<u32>()
        .map_err(|error| E::custom(format!("invalid integer value `{raw}`: {error}")))
}

pub fn deserialize_u32_from_string_or_number<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    parse_u32(StringOrNumber::deserialize(deserializer)?)
}

pub fn deserialize_option_u32_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrNumber>::deserialize(deserializer)?
        .map(parse_u32)
        .transpose()
}

pub mod string_contract {
    use super::*;

    pub fn serialize_u32<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn serialize_option_u32<S>(value: &Option<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serialize_u32(value, serializer),
            None => serializer.serialize_none(),
        }
    }
}
