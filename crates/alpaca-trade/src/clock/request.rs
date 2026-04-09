use alpaca_core::QueryWriter;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetV3Request {
    pub markets: Option<Vec<String>>,
    pub time: Option<String>,
}

impl GetV3Request {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        if let Some(markets) = validate_optional_csv_text("markets", self.markets)? {
            query.push_csv("markets", markets);
        }
        query.push_opt("time", validate_optional_text("time", self.time)?);
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

fn validate_required_text(name: &str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{name} must not be empty or whitespace-only"
        )));
    }

    Ok(trimmed.to_owned())
}
