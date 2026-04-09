use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub class: String,
    pub exchange: String,
    pub symbol: String,
    pub name: String,
    pub status: String,
    pub tradable: bool,
    pub marginable: bool,
    pub shortable: bool,
    pub easy_to_borrow: bool,
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
    pub attributes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsCorporatesResponse {
    pub us_corporates: Vec<UsCorporateBond>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsTreasuriesResponse {
    pub us_treasuries: Vec<UsTreasuryBond>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsCorporateBond {
    pub cusip: String,
    pub isin: String,
    pub bond_status: String,
    pub tradable: bool,
    pub marginable: bool,
    pub reissue_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub reissue_size: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub reissue_price: Option<Decimal>,
    pub issue_date: String,
    pub maturity_date: Option<String>,
    pub country_domicile: String,
    pub ticker: String,
    pub seniority: String,
    pub issuer: String,
    pub sector: String,
    pub description: String,
    pub description_short: String,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub coupon: Decimal,
    pub coupon_type: String,
    pub coupon_frequency: String,
    pub first_coupon_date: Option<String>,
    pub next_coupon_date: Option<String>,
    pub last_coupon_date: Option<String>,
    pub perpetual: bool,
    pub day_count: String,
    pub dated_date: String,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub issue_size: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub issue_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub issue_minimum_denomination: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub par_value: Decimal,
    pub callable: bool,
    pub next_call_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub next_call_price: Option<Decimal>,
    pub puttable: bool,
    pub convertible: bool,
    pub reg_s: bool,
    pub sp_rating: Option<String>,
    pub sp_rating_date: Option<String>,
    pub sp_creditwatch: Option<String>,
    pub sp_creditwatch_date: Option<String>,
    pub sp_outlook: Option<String>,
    pub sp_outlook_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_micro_buy: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_micro_sell: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_micro_aggregate: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_retail_buy: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_retail_sell: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_retail_aggregate: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_institutional_buy: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_institutional_sell: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub liquidity_institutional_aggregate: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_price: Option<Decimal>,
    pub close_price_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_yield_to_maturity: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_yield_to_worst: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub accrued_interest: Option<Decimal>,
    pub call_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsTreasuryBond {
    pub cusip: String,
    pub isin: String,
    pub bond_status: String,
    pub tradable: bool,
    pub subtype: String,
    pub issue_date: String,
    pub maturity_date: String,
    pub description: String,
    pub description_short: String,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_price: Option<Decimal>,
    pub close_price_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_yield_to_maturity: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_option_decimal"
    )]
    pub close_yield_to_worst: Option<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::number_contract::serialize_decimal"
    )]
    pub coupon: Decimal,
    pub coupon_type: String,
    pub coupon_frequency: String,
    pub first_coupon_date: Option<String>,
    pub next_coupon_date: Option<String>,
    pub last_coupon_date: Option<String>,
}
