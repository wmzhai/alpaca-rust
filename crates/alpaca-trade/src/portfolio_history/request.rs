use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetRequest {
    pub period: Option<String>,
    pub timeframe: Option<String>,
    pub intraday_reporting: Option<String>,
    pub start: Option<String>,
    pub pnl_reset: Option<String>,
    pub end: Option<String>,
    pub extended_hours: Option<String>,
    pub cashflow_types: Option<String>,
}

impl GetRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt("period", validate_optional_text("period", self.period)?);
        query.push_opt(
            "timeframe",
            validate_optional_text("timeframe", self.timeframe)?,
        );
        query.push_opt(
            "intraday_reporting",
            validate_optional_text("intraday_reporting", self.intraday_reporting)?,
        );
        query.push_opt("start", validate_optional_text("start", self.start)?);
        query.push_opt(
            "pnl_reset",
            validate_optional_text("pnl_reset", self.pnl_reset)?,
        );
        query.push_opt("end", validate_optional_text("end", self.end)?);
        query.push_opt(
            "extended_hours",
            validate_optional_text("extended_hours", self.extended_hours)?,
        );
        query.push_opt(
            "cashflow_types",
            validate_optional_text("cashflow_types", self.cashflow_types)?,
        );
        Ok(query.finish())
    }
}

fn validate_optional_text(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
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
