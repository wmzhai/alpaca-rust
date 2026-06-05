use serde::{Deserialize, Deserializer, de};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Number(serde_json::Number),
}

fn parse_f64<E>(value: StringOrNumber) -> Result<f64, E>
where
    E: de::Error,
{
    let raw = match value {
        StringOrNumber::String(value) => value,
        StringOrNumber::Number(value) => value.to_string(),
    };
    let number = raw
        .parse::<f64>()
        .map_err(|error| E::custom(format!("invalid float value `{raw}`: {error}")))?;

    if number.is_finite() {
        Ok(number)
    } else {
        Err(E::custom(format!("float value must be finite: `{raw}`")))
    }
}

pub fn round(value: f64, scale: u32) -> f64 {
    if !value.is_finite() {
        return value;
    }

    let multiplier = 10_f64.powi(scale as i32);
    (value * multiplier).round() / multiplier
}

pub fn deserialize_option_f64_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrNumber>::deserialize(deserializer)?
        .map(parse_f64)
        .transpose()
}
