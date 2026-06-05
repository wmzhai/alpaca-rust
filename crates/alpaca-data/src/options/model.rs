use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    pub t: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub o: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub h: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub l: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub c: Option<Decimal>,
    pub v: Option<u64>,
    pub n: Option<u64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub vw: Option<Decimal>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub t: Option<String>,
    pub bx: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub bp: Option<Decimal>,
    pub bs: Option<u64>,
    pub ax: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub ap: Option<Decimal>,
    #[serde(rename = "as")]
    pub r#as: Option<u64>,
    pub c: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub t: Option<String>,
    pub x: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub p: Option<Decimal>,
    pub s: Option<u64>,
    pub c: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Greeks {
    #[serde(
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub delta: Option<f64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub gamma: Option<f64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub rho: Option<f64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub theta: Option<f64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub vega: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(rename = "latestTrade")]
    pub latest_trade: Option<Trade>,
    #[serde(rename = "latestQuote")]
    pub latest_quote: Option<Quote>,
    #[serde(rename = "minuteBar")]
    pub minute_bar: Option<Bar>,
    #[serde(rename = "dailyBar")]
    pub daily_bar: Option<Bar>,
    #[serde(rename = "prevDailyBar")]
    pub prev_daily_bar: Option<Bar>,
    pub greeks: Option<Greeks>,
    #[serde(
        rename = "impliedVolatility",
        default,
        deserialize_with = "alpaca_core::float::deserialize_option_f64_from_string_or_number"
    )]
    pub implied_volatility: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("value should deserialize");
        assert!(
            (actual - expected).abs() <= 1e-12,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn option_snapshot_greeks_and_iv_deserialize_as_floats() {
        let snapshot: Snapshot = serde_json::from_value(json!({
            "greeks": {
                "delta": "0.123456789123",
                "gamma": 0.012345678912,
                "rho": "0.034567891234",
                "theta": -0.045678912345,
                "vega": "0.156789123456"
            },
            "impliedVolatility": "0.267891234567"
        }))
        .expect("snapshot should deserialize");
        let greeks = snapshot.greeks.expect("greeks should deserialize");

        assert_close(greeks.delta, 0.123456789123);
        assert_close(greeks.gamma, 0.012345678912);
        assert_close(greeks.rho, 0.034567891234);
        assert_close(greeks.theta, -0.045678912345);
        assert_close(greeks.vega, 0.156789123456);
        assert_close(snapshot.implied_volatility, 0.267891234567);
    }
}
