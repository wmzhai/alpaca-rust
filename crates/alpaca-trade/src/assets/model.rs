use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub class: AssetClass,
    pub exchange: Exchange,
    pub symbol: String,
    pub name: String,
    pub status: AssetStatus,
    pub tradable: bool,
    pub marginable: bool,
    pub shortable: bool,
    pub easy_to_borrow: bool,
    pub borrow_status: Option<BorrowStatus>,
    pub fractionable: bool,
    pub cusip: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub maintenance_margin_requirement: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub margin_requirement_long: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub margin_requirement_short: Option<Decimal>,
    pub attributes: Option<Vec<AssetAttribute>>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub min_order_size: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub min_trade_increment: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub price_increment: Option<Decimal>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Active,
    Inactive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    #[default]
    UsEquity,
    UsOption,
    Crypto,
    CryptoPerp,
    Treasury,
    Corporate,
    GlobalEquity,
    UsIndex,
    UsEquityChain,
    Ipo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exchange {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorrowStatus {
    EasyToBorrow,
    HardToBorrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetAttribute {
    PtpNoException,
    PtpWithException,
    Ipo,
    HasOptions,
    OptionsLateClose,
    FractionalEhEnabled,
    OvernightTradable,
    OvernightHalted,
}

macro_rules! impl_display {
    ($type:ty { $($variant:ident => $value:literal,)+ }) => {
        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(match self {
                    $(Self::$variant => $value,)+
                })
            }
        }
    };
}

impl_display!(AssetStatus {
    Active => "active",
    Inactive => "inactive",
});

impl_display!(AssetClass {
    UsEquity => "us_equity",
    UsOption => "us_option",
    Crypto => "crypto",
    CryptoPerp => "crypto_perp",
    Treasury => "treasury",
    Corporate => "corporate",
    GlobalEquity => "global_equity",
    UsIndex => "us_index",
    UsEquityChain => "us_equity_chain",
    Ipo => "ipo",
});

impl_display!(Exchange {
    Amex => "AMEX",
    Arca => "ARCA",
    Bats => "BATS",
    Nyse => "NYSE",
    Nasdaq => "NASDAQ",
    NyseArca => "NYSEARCA",
    Otc => "OTC",
    Crypto => "CRYPTO",
});

impl_display!(AssetAttribute {
    PtpNoException => "ptp_no_exception",
    PtpWithException => "ptp_with_exception",
    Ipo => "ipo",
    HasOptions => "has_options",
    OptionsLateClose => "options_late_close",
    FractionalEhEnabled => "fractional_eh_enabled",
    OvernightTradable => "overnight_tradable",
    OvernightHalted => "overnight_halted",
});
