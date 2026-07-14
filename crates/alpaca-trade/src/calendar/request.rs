use std::fmt;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub start: Option<String>,
    pub end: Option<String>,
    pub date_type: Option<DateType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DateType {
    Trading,
    Settlement,
}

impl fmt::Display for DateType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Trading => "TRADING",
            Self::Settlement => "SETTLEMENT",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarTimezone {
    #[serde(rename = "UTC")]
    Utc,
}

impl fmt::Display for CalendarTimezone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UTC")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Market {
    BMO,
    BNYM,
    BOATS,
    IEX,
    IEXG,
    NASDAQ,
    NYSE,
    OCEA,
    OPRA,
    OTC,
    OTCM,
    SIFMA,
    XNAS,
    XNYS,
    CEUX,
    CHIX,
    ISE,
    LSE,
    MTA,
    MTAA,
    XAMS,
    XBRU,
    XDUB,
    XETR,
    XETRA,
    XLIS,
    XLON,
    XPAR,
    HKEX,
    JPX,
    TADAWUL,
    XHKG,
    XSAU,
    XTKS,
}

impl fmt::Display for Market {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::BMO => "BMO",
            Self::BNYM => "BNYM",
            Self::BOATS => "BOATS",
            Self::IEX => "IEX",
            Self::IEXG => "IEXG",
            Self::NASDAQ => "NASDAQ",
            Self::NYSE => "NYSE",
            Self::OCEA => "OCEA",
            Self::OPRA => "OPRA",
            Self::OTC => "OTC",
            Self::OTCM => "OTCM",
            Self::SIFMA => "SIFMA",
            Self::XNAS => "XNAS",
            Self::XNYS => "XNYS",
            Self::CEUX => "CEUX",
            Self::CHIX => "CHIX",
            Self::ISE => "ISE",
            Self::LSE => "LSE",
            Self::MTA => "MTA",
            Self::MTAA => "MTAA",
            Self::XAMS => "XAMS",
            Self::XBRU => "XBRU",
            Self::XDUB => "XDUB",
            Self::XETR => "XETR",
            Self::XETRA => "XETRA",
            Self::XLIS => "XLIS",
            Self::XLON => "XLON",
            Self::XPAR => "XPAR",
            Self::HKEX => "HKEX",
            Self::JPX => "JPX",
            Self::TADAWUL => "TADAWUL",
            Self::XHKG => "XHKG",
            Self::XSAU => "XSAU",
            Self::XTKS => "XTKS",
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListV3Request {
    pub start: Option<String>,
    pub end: Option<String>,
    pub timezone: Option<CalendarTimezone>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let (start, end) = validate_date_range(self.start, self.end)?;
        let mut query = QueryWriter::default();
        query.push_opt("start", start);
        query.push_opt("end", end);
        query.push_opt("date_type", self.date_type);
        Ok(query.finish())
    }
}

impl ListV3Request {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let (start, end) = validate_date_range(self.start, self.end)?;
        let mut query = QueryWriter::default();
        query.push_opt("start", start);
        query.push_opt("end", end);
        query.push_opt("timezone", self.timezone);
        Ok(query.finish())
    }
}

fn validate_date_range(
    start: Option<String>,
    end: Option<String>,
) -> Result<(Option<String>, Option<String>), Error> {
    let start = validate_optional_date("start", start)?;
    let end = validate_optional_date("end", end)?;
    if start
        .as_ref()
        .zip(end.as_ref())
        .is_some_and(|(start, end)| start > end)
    {
        return Err(Error::InvalidRequest(
            "start must not be after end".to_owned(),
        ));
    }
    Ok((start, end))
}

fn validate_optional_date(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| {
            let value = validate_required_text(name, &value)?;
            NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                .map_err(|_| Error::InvalidRequest(format!("{name} must use YYYY-MM-DD format")))?;
            Ok(value)
        })
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
