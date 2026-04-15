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
pub struct BarsSingleRequest {
    pub symbol: String,
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
pub struct AuctionsSingleRequest {
    pub symbol: String,
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
pub struct QuotesSingleRequest {
    pub symbol: String,
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
pub struct TradesSingleRequest {
    pub symbol: String,
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
pub struct LatestBarRequest {
    pub symbol: String,
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
pub struct LatestQuoteRequest {
    pub symbol: String,
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
pub struct LatestTradeRequest {
    pub symbol: String,
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
pub struct SnapshotRequest {
    pub symbol: String,
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
}

impl BarsSingleRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
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
}

impl AuctionsSingleRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
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
}

impl QuotesSingleRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
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
}

impl TradesSingleRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
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
}

impl LatestBarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }
}

impl LatestBarRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_single_query(self.feed, self.currency)
    }
}

impl LatestQuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }
}

impl LatestQuoteRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_single_query(self.feed, self.currency)
    }
}

impl LatestTradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }
}

impl LatestTradeRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_single_query(self.feed, self.currency)
    }
}

impl SnapshotsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbols(&self.symbols)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_batch_query(self.symbols, self.feed, self.currency)
    }
}

impl SnapshotRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.symbol, "symbol")
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_single_query(self.feed, self.currency)
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

impl PaginatedRequest for BarsSingleRequest {
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

impl PaginatedRequest for AuctionsSingleRequest {
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

impl PaginatedRequest for QuotesSingleRequest {
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

impl PaginatedRequest for TradesSingleRequest {
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

fn latest_single_query(
    feed: Option<DataFeed>,
    currency: Option<Currency>,
) -> Vec<(String, String)> {
    let mut query = QueryWriter::default();
    query.push_opt("feed", feed);
    query.push_opt("currency", currency);
    query.finish()
}

fn validate_required_symbol(symbol: &str, field_name: &str) -> Result<(), Error> {
    if normalized_stock_symbol(symbol).is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{field_name} is invalid: must not be empty or whitespace-only"
        )));
    }

    Ok(())
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
