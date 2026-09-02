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
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub v: Option<Decimal>,
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
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub bp: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub bs: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub ap: Option<Decimal>,
    #[serde(
        rename = "as",
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub r#as: Option<Decimal>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub t: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub p: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub s: Option<Decimal>,
    pub i: Option<i64>,
    pub tks: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderbookEntry {
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub p: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub s: Option<Decimal>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Orderbook {
    pub t: Option<String>,
    #[serde(default)]
    pub b: Vec<OrderbookEntry>,
    #[serde(default)]
    pub a: Vec<OrderbookEntry>,
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
