use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub status: Option<String>,
    pub asset_class: Option<String>,
    pub exchange: Option<String>,
    pub attributes: Option<Vec<String>>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt("status", validate_optional_text("status", self.status)?);
        query.push_opt(
            "asset_class",
            validate_optional_text("asset_class", self.asset_class)?,
        );
        query.push_opt(
            "exchange",
            validate_optional_text("exchange", self.exchange)?,
        );
        if let Some(attributes) = validate_optional_csv_text("attributes", self.attributes)? {
            query.push_csv("attributes", attributes);
        }
        Ok(query.finish())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsCorporatesRequest {
    pub bond_status: Option<String>,
    pub isins: Option<Vec<String>>,
    pub cusips: Option<Vec<String>>,
    pub tickers: Option<Vec<String>>,
}

impl UsCorporatesRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt(
            "bond_status",
            validate_optional_text("bond_status", self.bond_status)?,
        );
        if let Some(isins) = validate_optional_csv_text("isins", self.isins)? {
            query.push_csv("isins", isins);
        }
        if let Some(cusips) = validate_optional_csv_text("cusips", self.cusips)? {
            query.push_csv("cusips", cusips);
        }
        if let Some(tickers) = validate_optional_csv_text("tickers", self.tickers)? {
            query.push_csv("tickers", tickers);
        }
        Ok(query.finish())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsTreasuriesRequest {
    pub subtype: Option<String>,
    pub bond_status: Option<String>,
    pub cusips: Option<Vec<String>>,
    pub isins: Option<Vec<String>>,
}

impl UsTreasuriesRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt("subtype", validate_optional_text("subtype", self.subtype)?);
        query.push_opt(
            "bond_status",
            validate_optional_text("bond_status", self.bond_status)?,
        );
        if let Some(cusips) = validate_optional_csv_text("cusips", self.cusips)? {
            query.push_csv("cusips", cusips);
        }
        if let Some(isins) = validate_optional_csv_text("isins", self.isins)? {
            query.push_csv("isins", isins);
        }
        Ok(query.finish())
    }
}

pub(crate) fn validate_symbol_or_asset_id(symbol_or_asset_id: &str) -> Result<String, Error> {
    let trimmed = symbol_or_asset_id.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(
            "symbol_or_asset_id must not be empty or whitespace-only".to_owned(),
        ));
    }
    if trimmed.contains('/') {
        return Err(Error::InvalidRequest(
            "symbol_or_asset_id must not contain `/`".to_owned(),
        ));
    }

    Ok(trimmed.to_owned())
}

fn validate_optional_text(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
}

fn validate_optional_csv_text(
    name: &str,
    values: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, Error> {
    match values {
        None => Ok(None),
        Some(values) if values.is_empty() => Ok(None),
        Some(values) => values
            .into_iter()
            .map(|value| validate_required_text(name, &value))
            .collect::<Result<Vec<_>, Error>>()
            .map(Some),
    }
}

fn validate_required_text(name: &str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{name} must not be empty or whitespace-only"
        )));
    }

    Ok(trimmed.to_owned())
}
