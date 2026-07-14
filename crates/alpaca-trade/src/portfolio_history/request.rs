use std::fmt;

use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetRequest {
    pub period: Option<String>,
    pub timeframe: Option<Timeframe>,
    pub intraday_reporting: Option<IntradayReporting>,
    pub start: Option<String>,
    pub pnl_reset: Option<PnlReset>,
    pub end: Option<String>,
    pub extended_hours: Option<String>,
    pub cashflow_types: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timeframe {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    OneDay,
}

impl fmt::Display for Timeframe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OneMinute => "1Min",
            Self::FiveMinutes => "5Min",
            Self::FifteenMinutes => "15Min",
            Self::OneHour => "1H",
            Self::OneDay => "1D",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntradayReporting {
    MarketHours,
    ExtendedHours,
    Continuous,
}

impl fmt::Display for IntradayReporting {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MarketHours => "market_hours",
            Self::ExtendedHours => "extended_hours",
            Self::Continuous => "continuous",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PnlReset {
    NoReset,
    PerDay,
}

impl fmt::Display for PnlReset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoReset => "no_reset",
            Self::PerDay => "per_day",
        })
    }
}

impl GetRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let range_fields = [
            self.period.is_some(),
            self.start.is_some(),
            self.end.is_some(),
        ]
        .into_iter()
        .filter(|is_some| *is_some)
        .count();
        if range_fields > 2 {
            return Err(Error::InvalidRequest(
                "only two of period, start, and end may be specified".to_owned(),
            ));
        }

        let mut query = QueryWriter::default();
        query.push_opt("period", validate_optional_text("period", self.period)?);
        query.push_opt("timeframe", self.timeframe);
        query.push_opt("intraday_reporting", self.intraday_reporting);
        query.push_opt("start", validate_optional_text("start", self.start)?);
        query.push_opt("pnl_reset", self.pnl_reset);
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
