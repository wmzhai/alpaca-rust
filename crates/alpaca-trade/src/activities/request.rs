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
    #[must_use]
    pub fn for_types(activity_types: &[&str], after_date: Option<&str>) -> Self {
        Self {
            activity_types: Some(activity_types.iter().map(|value| (*value).to_owned()).collect()),
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
        Self::for_types(&["OPASN", "OPEXP"], after_date)
    }

    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        if let Some(activity_types) = validate_activity_types(self.activity_types)? {
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

fn validate_activity_type(activity_type: &str) -> Result<String, Error> {
    let trimmed = validate_required_text("activity_types", activity_type)?;
    if trimmed.contains('/') {
        return Err(Error::InvalidRequest(
            "activity_type must not contain `/`".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::ListRequest;

    #[test]
    fn for_types_sets_filters_and_page_size() {
        let request = ListRequest::for_types(&["DIV", "DIVNRA"], Some("2026-04-01"));

        assert_eq!(
            request.activity_types,
            Some(vec!["DIV".to_string(), "DIVNRA".to_string()])
        );
        assert_eq!(request.after.as_deref(), Some("2026-04-01"));
        assert_eq!(request.page_size, Some(100));
    }

    #[test]
    fn option_records_request_sets_default_filters() {
        let request = ListRequest::option_records(None);

        assert_eq!(
            request.activity_types,
            Some(vec!["OPASN".to_string(), "OPEXP".to_string()])
        );
        assert_eq!(request.page_size, Some(100));
        assert_eq!(request.after, None);
    }

    #[test]
    fn option_records_request_preserves_explicit_after_date() {
        let request = ListRequest::option_records(Some("2026-04-01"));

        assert_eq!(request.after.as_deref(), Some("2026-04-01"));
    }
}
