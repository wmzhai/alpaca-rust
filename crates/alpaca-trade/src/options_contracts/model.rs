use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use alpaca_core::Error;
use alpaca_core::pagination::PaginatedResponse;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractStatus {
    Active,
    Inactive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractType {
    Call,
    Put,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractStyle {
    American,
    European,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliverableType {
    Cash,
    Equity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliverableSettlementType {
    #[serde(rename = "T+0")]
    TPlus0,
    #[serde(rename = "T+1")]
    TPlus1,
    #[serde(rename = "T+2")]
    TPlus2,
    #[serde(rename = "T+3")]
    TPlus3,
    #[serde(rename = "T+4")]
    TPlus4,
    #[serde(rename = "T+5")]
    TPlus5,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliverableSettlementMethod {
    #[serde(rename = "BTOB")]
    Btob,
    #[serde(rename = "CADF")]
    Cadf,
    #[serde(rename = "CAFX")]
    Cafx,
    #[serde(rename = "CCC")]
    Ccc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListResponse {
    pub option_contracts: Vec<OptionContract>,
    pub next_page_token: Option<String>,
}

impl PaginatedResponse for ListResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
        self.option_contracts.extend(next.option_contracts);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionContract {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub status: ContractStatus,
    pub tradable: bool,
    pub expiration_date: String,
    pub root_symbol: Option<String>,
    pub underlying_symbol: String,
    pub underlying_asset_id: String,
    #[serde(rename = "type")]
    pub r#type: ContractType,
    pub style: ContractStyle,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub strike_price: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub multiplier: Decimal,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub size: Decimal,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub open_interest: Option<Decimal>,
    pub open_interest_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub close_price: Option<Decimal>,
    pub close_price_date: Option<String>,
    pub deliverables: Option<Vec<OptionDeliverable>>,
    pub ppind: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionDeliverable {
    #[serde(rename = "type")]
    pub r#type: DeliverableType,
    pub symbol: String,
    pub asset_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub amount: Option<Decimal>,
    #[serde(
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal"
    )]
    pub allocation_percentage: Decimal,
    pub settlement_type: DeliverableSettlementType,
    pub settlement_method: DeliverableSettlementMethod,
    pub delayed_settlement: bool,
}
