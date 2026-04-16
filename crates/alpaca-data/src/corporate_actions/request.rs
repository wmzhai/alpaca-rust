use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;
use crate::symbols::display_stock_symbol;

use super::{CorporateActionType, Sort};

#[derive(Clone, Debug, Default)]
pub struct ListRequest {
    pub symbols: Option<Vec<String>>,
    pub cusips: Option<Vec<String>>,
    pub types: Option<Vec<CorporateActionType>>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub ids: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

impl ListRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_limit(self.limit, 1, 1_000)?;
        validate_optional_identifiers(self.symbols.as_deref(), "symbols")?;
        validate_optional_identifiers(self.cusips.as_deref(), "cusips")?;
        validate_optional_identifiers(self.ids.as_deref(), "ids")?;

        if let Some(types) = &self.types
            && types.is_empty()
        {
            return Err(Error::InvalidRequest(
                "types are invalid: must not be empty when provided".to_owned(),
            ));
        }

        if self.ids.is_some()
            && (self.symbols.is_some()
                || self.cusips.is_some()
                || self.types.is_some()
                || self.start.is_some()
                || self.end.is_some())
        {
            return Err(Error::InvalidRequest(
                "ids cannot be combined with other corporate actions filters".to_owned(),
            ));
        }

        Ok(())
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        if let Some(symbols) = self.symbols {
            query.push_csv(
                "symbols",
                symbols
                    .into_iter()
                    .map(|symbol| display_stock_symbol(&symbol))
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(cusips) = self.cusips {
            query.push_csv("cusips", cusips);
        }
        if let Some(types) = self.types {
            query.push_csv("types", types.into_iter().map(|value| value.to_string()));
        }
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        if let Some(ids) = self.ids {
            query.push_csv("ids", ids);
        }
        query.push_opt("limit", self.limit);
        query.push_opt("sort", self.sort);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }
}

impl PaginatedRequest for ListRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

fn validate_optional_identifiers(values: Option<&[String]>, field_name: &str) -> Result<(), Error> {
    let Some(values) = values else {
        return Ok(());
    };

    if values.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{field_name} are invalid: must not be empty when provided"
        )));
    }

    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(Error::InvalidRequest(format!(
            "{field_name} are invalid: must not contain empty or whitespace-only entries"
        )));
    }

    Ok(())
}

fn validate_limit(limit: Option<u32>, min: u32, max: u32) -> Result<(), Error> {
    if let Some(limit) = limit
        && !(min..=max).contains(&limit)
    {
        return Err(Error::InvalidRequest(format!(
            "limit must be between {min} and {max}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CorporateActionType, ListRequest};

    #[test]
    fn list_request_normalizes_stock_symbols_in_query() {
        let query = ListRequest {
            symbols: Some(vec![" brk/b ".to_owned(), "aapl".to_owned()]),
            cusips: None,
            types: Some(vec![CorporateActionType::CashDividend]),
            start: Some("2025-01-01".to_owned()),
            end: Some("2025-01-31".to_owned()),
            ids: None,
            limit: Some(100),
            sort: None,
            page_token: None,
        }
        .into_query();

        assert!(
            query
                .iter()
                .any(|(key, value)| key == "symbols" && value == "BRK.B,AAPL"),
            "corporate actions query should normalize stock symbols: {query:?}"
        );
    }
}
