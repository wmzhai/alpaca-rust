use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;

use super::Sort;

#[derive(Clone, Debug, Default)]
pub struct ListRequest {
    pub start: Option<String>,
    pub end: Option<String>,
    pub sort: Option<Sort>,
    pub symbols: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub include_content: Option<bool>,
    pub exclude_contentless: Option<bool>,
    pub page_token: Option<String>,
}

impl ListRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_limit(self.limit, 1, 50)?;

        if let Some(symbols) = &self.symbols {
            validate_symbols(symbols)?;
        }

        Ok(())
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("sort", self.sort);
        if let Some(symbols) = self.symbols {
            query.push_csv("symbols", symbols);
        }
        query.push_opt("limit", self.limit);
        query.push_opt("include_content", self.include_content);
        query.push_opt("exclude_contentless", self.exclude_contentless);
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

fn validate_symbols(symbols: &[String]) -> Result<(), Error> {
    if symbols.is_empty() {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not be empty when provided".to_owned(),
        ));
    }

    if symbols.iter().any(|symbol| symbol.trim().is_empty()) {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not contain empty or whitespace-only entries".to_owned(),
        ));
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
