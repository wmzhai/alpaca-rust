use serde::Serialize;

use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CreateRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<String>>,
}

impl CreateRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    fn validate(&self) -> Result<(), Error> {
        validate_required_text("name", &self.name)?;
        validate_optional_symbols(self.symbols.as_ref())?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpdateRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<String>>,
}

impl UpdateRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    fn validate(&self) -> Result<(), Error> {
        validate_required_text("name", &self.name)?;
        validate_optional_symbols(self.symbols.as_ref())?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AddAssetRequest {
    pub symbol: String,
}

impl AddAssetRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    fn validate(&self) -> Result<(), Error> {
        validate_required_text("symbol", &self.symbol)?;
        Ok(())
    }
}

pub(crate) fn name_query(name: &str) -> Result<Vec<(String, String)>, Error> {
    let mut query = QueryWriter::default();
    query.push("name", validate_required_text("name", name)?);
    Ok(query.finish())
}

pub(crate) fn validate_watchlist_id(watchlist_id: &str) -> Result<String, Error> {
    validate_required_path_segment("watchlist_id", watchlist_id)
}

pub(crate) fn validate_symbol(symbol: &str) -> Result<String, Error> {
    validate_required_path_segment("symbol", symbol)
}

fn validate_optional_symbols(symbols: Option<&Vec<String>>) -> Result<(), Error> {
    let Some(symbols) = symbols else {
        return Ok(());
    };
    if symbols.is_empty() {
        return Err(Error::InvalidRequest(
            "symbols must contain at least one symbol when provided".to_owned(),
        ));
    }

    for symbol in symbols {
        validate_required_text("symbols", symbol)?;
    }

    Ok(())
}

fn validate_required_path_segment(name: &str, value: &str) -> Result<String, Error> {
    let value = validate_required_text(name, value)?;
    if value.contains('/') {
        return Err(Error::InvalidRequest(format!(
            "{name} must not contain `/`"
        )));
    }

    Ok(value)
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
