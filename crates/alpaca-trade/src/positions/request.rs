use rust_decimal::Decimal;
use serde::Serialize;

use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CloseAllRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_orders: Option<bool>,
}

impl CloseAllRequest {
    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("cancel_orders", self.cancel_orders);
        query.finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ClosePositionRequest {
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub qty: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub percentage: Option<Decimal>,
}

impl ClosePositionRequest {
    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("qty", self.qty);
        query.push_opt("percentage", self.percentage);
        query.finish()
    }
}

pub(crate) fn validate_symbol_or_asset_id(symbol_or_asset_id: &str) -> Result<String, Error> {
    validate_required_path_segment("symbol_or_asset_id", symbol_or_asset_id)
}

pub(crate) fn validate_symbol_or_contract_id(symbol_or_contract_id: &str) -> Result<String, Error> {
    validate_required_path_segment("symbol_or_contract_id", symbol_or_contract_id)
}

fn validate_required_path_segment(name: &str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{name} must not be empty or whitespace-only"
        )));
    }
    if trimmed != value {
        return Err(Error::InvalidRequest(format!(
            "{name} must not contain leading or trailing whitespace"
        )));
    }
    if trimmed.contains('/') {
        return Err(Error::InvalidRequest(format!(
            "{name} must not contain `/`"
        )));
    }

    Ok(trimmed.to_owned())
}
