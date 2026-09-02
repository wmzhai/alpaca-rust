use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;

use super::{Location, Sort, TimeFrame};

#[derive(Clone, Debug, Default)]
pub struct BarsRequest {
    pub location: Location,
    pub symbols: Vec<String>,
    pub timeframe: TimeFrame,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct QuotesRequest {
    pub location: Location,
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TradesRequest {
    pub location: Location,
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestBarsRequest {
    pub location: Location,
    pub symbols: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestQuotesRequest {
    pub location: Location,
    pub symbols: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestTradesRequest {
    pub location: Location,
    pub symbols: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestOrderbooksRequest {
    pub location: Location,
    pub symbols: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotsRequest {
    pub location: Location,
    pub symbols: Vec<String>,
}

impl BarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_crypto_symbols(&self.symbols));
        query.push_opt("timeframe", Some(self.timeframe));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("sort", self.sort);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }
}

impl QuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_crypto_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("sort", self.sort);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }
}

impl TradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_crypto_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("sort", self.sort);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }
}

impl LatestBarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols)
    }
}

impl LatestQuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols)
    }
}

impl LatestTradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols)
    }
}

impl LatestOrderbooksRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols)
    }
}

impl SnapshotsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols)
    }
}

impl PaginatedRequest for BarsRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

impl PaginatedRequest for QuotesRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

impl PaginatedRequest for TradesRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

fn latest_batch_query(symbols: Vec<String>) -> Vec<(String, String)> {
    let mut query = QueryWriter::default();
    query.push_csv("symbols", normalized_crypto_symbols(&symbols));
    query.finish()
}

fn validate_required_symbols(symbols: &[String]) -> Result<(), Error> {
    if symbols.is_empty() {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not be empty".to_owned(),
        ));
    }

    if symbols
        .iter()
        .any(|symbol| normalized_crypto_symbol(symbol).is_empty())
    {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not contain empty or whitespace-only entries".to_owned(),
        ));
    }

    Ok(())
}

fn normalized_crypto_symbol(symbol: &str) -> String {
    symbol.trim().to_owned()
}

fn normalized_crypto_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .map(|symbol| normalized_crypto_symbol(symbol))
        .collect()
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
