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
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        if self.qty.is_some() && self.percentage.is_some() {
            return Err(Error::InvalidRequest(
                "qty and percentage are mutually exclusive".to_owned(),
            ));
        }

        if let Some(qty) = self.qty {
            validate_close_amount("qty", qty, None)?;
        }
        if let Some(percentage) = self.percentage {
            validate_close_amount("percentage", percentage, Some(Decimal::new(100, 0)))?;
        }

        let mut query = QueryWriter::default();
        query.push_opt("qty", self.qty);
        query.push_opt("percentage", self.percentage);
        Ok(query.finish())
    }
}

fn validate_close_amount(
    name: &str,
    value: Decimal,
    maximum: Option<Decimal>,
) -> Result<(), Error> {
    if value <= Decimal::ZERO {
        return Err(Error::InvalidRequest(format!(
            "{name} must be greater than 0"
        )));
    }
    if maximum.is_some_and(|maximum| value > maximum) {
        return Err(Error::InvalidRequest(format!("{name} must be at most 100")));
    }
    if value.scale() > 9 {
        return Err(Error::InvalidRequest(format!(
            "{name} must have at most 9 decimal places"
        )));
    }

    Ok(())
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
