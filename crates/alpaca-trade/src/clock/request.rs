use chrono::DateTime;

use alpaca_core::QueryWriter;

use crate::{Error, calendar::Market};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetV3Request {
    pub markets: Option<Vec<Market>>,
    pub time: Option<String>,
}

impl GetV3Request {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        if let Some(markets) = validate_markets(self.markets)? {
            query.push_csv("markets", markets);
        }
        query.push_opt("time", validate_time(self.time)?);
        Ok(query.finish())
    }
}

fn validate_markets(values: Option<Vec<Market>>) -> Result<Option<Vec<Market>>, Error> {
    match values {
        None => Ok(None),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(
            "markets must contain at least one value".to_owned(),
        )),
        Some(values) => Ok(Some(values)),
    }
}

fn validate_time(value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| {
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(Error::InvalidRequest("time must not be empty".to_owned()));
            }
            DateTime::parse_from_rfc3339(&value)
                .map_err(|_| Error::InvalidRequest("time must use RFC3339 format".to_owned()))?;
            Ok(value)
        })
        .transpose()
}
