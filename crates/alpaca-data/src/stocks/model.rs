use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Auction {
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
pub struct DailyAuction {
    pub d: Option<String>,
    #[serde(default)]
    pub o: Vec<Auction>,
    #[serde(default)]
    pub c: Vec<Auction>,
}

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
    pub c: Option<Vec<String>>,
    pub z: Option<String>,
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
    pub i: Option<u64>,
    pub c: Option<Vec<String>>,
    pub z: Option<String>,
    pub u: Option<String>,
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
}
