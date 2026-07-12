use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;
use crate::symbols::display_stock_symbol;

use super::{Adjustment, AuctionFeed, Currency, DataFeed, Sort, Tape, TickType, TimeFrame};

#[derive(Clone, Debug, Default)]
pub struct BarsRequest {
    pub symbols: Vec<String>,
    pub timeframe: TimeFrame,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub adjustment: Option<Adjustment>,
    pub feed: Option<DataFeed>,
    pub sort: Option<Sort>,
    pub asof: Option<String>,
    pub currency: Option<Currency>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AuctionsRequest {
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub asof: Option<String>,
    pub feed: Option<AuctionFeed>,
    pub currency: Option<Currency>,
    pub page_token: Option<String>,
    pub sort: Option<Sort>,
}

#[derive(Clone, Debug, Default)]
pub struct QuotesRequest {
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub feed: Option<DataFeed>,
    pub sort: Option<Sort>,
    pub asof: Option<String>,
    pub currency: Option<Currency>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TradesRequest {
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub feed: Option<DataFeed>,
    pub sort: Option<Sort>,
    pub asof: Option<String>,
    pub currency: Option<Currency>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestBarsRequest {
    pub symbols: Vec<String>,
    pub feed: Option<DataFeed>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestQuotesRequest {
    pub symbols: Vec<String>,
    pub feed: Option<DataFeed>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestTradesRequest {
    pub symbols: Vec<String>,
    pub feed: Option<DataFeed>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotsRequest {
    pub symbols: Vec<String>,
    pub feed: Option<DataFeed>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default)]
pub struct ConditionCodesRequest {
    pub ticktype: TickType,
    pub tape: Tape,
}

impl BarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_stock_symbols(&self.symbols));
        query.push_opt("timeframe", Some(self.timeframe));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("adjustment", self.adjustment);
        query.push_opt("feed", self.feed);
        query.push_opt("sort", self.sort);
        query.push_opt("asof", self.asof);
        query.push_opt("currency", self.currency);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl AuctionsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_stock_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("asof", self.asof);
        query.push_opt("feed", self.feed);
        query.push_opt("currency", self.currency);
        query.push_opt("page_token", self.page_token);
        query.push_opt("sort", self.sort);
        query.finish()
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl QuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_stock_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("feed", self.feed);
        query.push_opt("sort", self.sort);
        query.push_opt("asof", self.asof);
        query.push_opt("currency", self.currency);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl TradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_stock_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("feed", self.feed);
        query.push_opt("sort", self.sort);
        query.push_opt("asof", self.asof);
        query.push_opt("currency", self.currency);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl LatestBarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl LatestQuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl LatestTradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl SnapshotsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }

    pub(crate) fn single_symbol(&self) -> Option<String> {
        normalized_single_stock_symbol(&self.symbols)
    }

    pub(crate) fn into_single_query(mut self) -> Vec<(String, String)> {
        self.symbols.clear();
        self.into_query()
    }
}

impl ConditionCodesRequest {
    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("tape", Some(self.tape));
        query.finish()
    }
}

impl PaginatedRequest for BarsRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

impl PaginatedRequest for AuctionsRequest {
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

fn latest_batch_query(
    symbols: Vec<String>,
    feed: Option<DataFeed>,
    currency: Option<Currency>,
) -> Vec<(String, String)> {
    let mut query = QueryWriter::default();
    query.push_csv("symbols", normalized_stock_symbols(&symbols));
    query.push_opt("feed", feed);
    query.push_opt("currency", currency);
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
        .any(|symbol| normalized_stock_symbol(symbol).is_empty())
    {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not contain empty or whitespace-only entries".to_owned(),
        ));
    }

    Ok(())
}

fn normalized_stock_symbol(symbol: &str) -> String {
    display_stock_symbol(symbol)
}

fn normalized_stock_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .map(|symbol| normalized_stock_symbol(symbol))
        .collect()
}

fn normalized_single_stock_symbol(symbols: &[String]) -> Option<String> {
    (symbols.len() == 1).then(|| encoded_stock_path_segment(&normalized_stock_symbol(&symbols[0])))
}

fn encoded_stock_path_segment(symbol: &str) -> String {
    let mut url = reqwest::Url::parse("https://example.invalid/")
        .expect("constant stock path encoding URL should parse");
    url.path_segments_mut()
        .expect("constant stock path encoding URL should support path segments")
        .push(symbol);
    url.path().trim_start_matches('/').to_owned()
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
