use std::fmt;

use chrono::{Duration, Utc};
use chrono_tz::America::New_York;
use serde::{Deserialize, Serialize};

use alpaca_core::QueryWriter;

use crate::{Error, orders::SortDirection};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub activity_types: Option<Vec<String>>,
    pub category: Option<ActivityCategory>,
    pub date: Option<String>,
    pub until: Option<String>,
    pub after: Option<String>,
    pub direction: Option<SortDirection>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListByTypeRequest {
    pub date: Option<String>,
    pub until: Option<String>,
    pub after: Option<String>,
    pub direction: Option<SortDirection>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    TradeActivity,
    NonTradeActivity,
}

impl fmt::Display for ActivityCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TradeActivity => "trade_activity",
            Self::NonTradeActivity => "non_trade_activity",
        })
    }
}

impl ListRequest {
    #[must_use]
    pub fn for_types(activity_types: &[&str], after_date: Option<&str>) -> Self {
        Self {
            activity_types: Some(
                activity_types
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
            ),
            category: None,
            date: None,
            until: None,
            after: after_date.map(ToOwned::to_owned),
            direction: None,
            page_size: Some(100),
            page_token: None,
        }
    }

    #[must_use]
    pub fn option_records(after_date: Option<&str>) -> Self {
        let after_date = after_date
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_option_records_after_date);
        Self::for_types(&["OPASN", "OPEXP"], Some(after_date.as_str()))
    }

    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        if self.activity_types.is_some() && self.category.is_some() {
            return Err(Error::InvalidRequest(
                "activity_types and category are mutually exclusive".to_owned(),
            ));
        }

        let mut query = QueryWriter::default();
        if let Some(activity_types) = validate_activity_types(self.activity_types)? {
            query.push_csv("activity_types", activity_types);
        }
        query.push_opt("category", self.category);
        append_page_query(
            &mut query,
            self.date,
            self.until,
            self.after,
            self.direction,
            self.page_size,
            self.page_token,
        )?;
        Ok(query.finish())
    }
}

impl ListByTypeRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        append_page_query(
            &mut query,
            self.date,
            self.until,
            self.after,
            self.direction,
            self.page_size,
            self.page_token,
        )?;
        Ok(query.finish())
    }
}

pub(crate) fn validate_activity_type(activity_type: &str) -> Result<String, Error> {
    const ACTIVITY_TYPES: &[&str] = &[
        "FILL", "TRANS", "MISC", "ACATC", "ACATS", "CFEE", "CGD", "CSD", "CSW", "DIV", "DIVCGL",
        "DIVCGS", "DIVFEE", "DIVFT", "DIVNRA", "DIVROC", "DIVTW", "DIVTXEX", "FEE", "INT",
        "INTNRA", "INTTW", "JNL", "JNLC", "JNLS", "MA", "NC", "OPASN", "OPCA", "OPCSH", "OPEXC",
        "OPEXP", "OPTRD", "PTC", "PTR", "REORG", "SPIN", "SPLIT", "FOPT",
    ];

    let trimmed = validate_required_text("activity_type", activity_type)?;
    if !ACTIVITY_TYPES.contains(&trimmed.as_str()) {
        return Err(Error::InvalidRequest(
            "activity_type must be a canonical Alpaca activity type".to_owned(),
        ));
    }

    Ok(trimmed)
}

fn validate_activity_types(values: Option<Vec<String>>) -> Result<Option<Vec<String>>, Error> {
    match values {
        None => Ok(None),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(format!(
            "activity_types must contain at least one value"
        ))),
        Some(values) => values
            .into_iter()
            .map(|value| validate_activity_type(&value))
            .collect::<Result<Vec<_>, Error>>()
            .map(Some),
    }
}

fn validate_optional_text(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
}

fn validate_page_size(value: Option<u32>) -> Result<Option<u32>, Error> {
    match value {
        Some(0) => Err(Error::InvalidRequest(
            "page_size must be greater than 0".to_owned(),
        )),
        Some(value) if value > 100 => Err(Error::InvalidRequest(
            "page_size must not exceed 100".to_owned(),
        )),
        _ => Ok(value),
    }
}

fn append_page_query(
    query: &mut QueryWriter,
    date: Option<String>,
    until: Option<String>,
    after: Option<String>,
    direction: Option<SortDirection>,
    page_size: Option<u32>,
    page_token: Option<String>,
) -> Result<(), Error> {
    query.push_opt("date", validate_optional_text("date", date)?);
    query.push_opt("until", validate_optional_text("until", until)?);
    query.push_opt("after", validate_optional_text("after", after)?);
    query.push_opt("direction", direction);
    query.push_opt("page_size", validate_page_size(page_size)?);
    query.push_opt(
        "page_token",
        validate_optional_text("page_token", page_token)?,
    );
    Ok(())
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

fn default_option_records_after_date() -> String {
    (Utc::now().with_timezone(&New_York).date_naive() - Duration::days(30)).to_string()
}
