use alpaca_core::QueryWriter;

use crate::{Error, orders::SortDirection};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub activity_types: Option<Vec<String>>,
    pub date: Option<String>,
    pub until: Option<String>,
    pub after: Option<String>,
    pub direction: Option<SortDirection>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        if let Some(activity_types) =
            validate_optional_csv_text("activity_types", self.activity_types)?
        {
            query.push_csv("activity_types", activity_types);
        }
        query.push_opt("date", validate_optional_text("date", self.date)?);
        query.push_opt("until", validate_optional_text("until", self.until)?);
        query.push_opt("after", validate_optional_text("after", self.after)?);
        query.push_opt("direction", self.direction);
        query.push_opt(
            "page_size",
            validate_positive_u32("page_size", self.page_size)?,
        );
        query.push_opt(
            "page_token",
            validate_optional_text("page_token", self.page_token)?,
        );
        Ok(query.finish())
    }
}

fn validate_optional_csv_text(
    name: &str,
    values: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, Error> {
    match values {
        None => Ok(None),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(format!(
            "{name} must contain at least one value"
        ))),
        Some(values) => values
            .into_iter()
            .map(|value| validate_required_text(name, &value))
            .collect::<Result<Vec<_>, Error>>()
            .map(Some),
    }
}

fn validate_optional_text(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
}

fn validate_positive_u32(name: &str, value: Option<u32>) -> Result<Option<u32>, Error> {
    match value {
        Some(0) => Err(Error::InvalidRequest(format!(
            "{name} must be greater than 0"
        ))),
        _ => Ok(value),
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
