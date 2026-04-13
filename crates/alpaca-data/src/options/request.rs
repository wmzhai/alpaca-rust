use rust_decimal::Decimal;

use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;
use crate::symbols::{option_contract_symbol, options_underlying_symbol};

use super::{ContractType, OptionsFeed, Sort, TickType, TimeFrame};

const MAX_OPTION_SYMBOLS_PER_REQUEST: usize = 100;

#[derive(Clone, Debug, Default)]
pub struct BarsRequest {
    pub symbols: Vec<String>,
    pub timeframe: TimeFrame,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TradesRequest {
    pub symbols: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<Sort>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestQuotesRequest {
    pub symbols: Vec<String>,
    pub feed: Option<OptionsFeed>,
}

#[derive(Clone, Debug, Default)]
pub struct LatestTradesRequest {
    pub symbols: Vec<String>,
    pub feed: Option<OptionsFeed>,
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotsRequest {
    pub symbols: Vec<String>,
    pub feed: Option<OptionsFeed>,
    pub limit: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ChainRequest {
    pub underlying_symbol: String,
    pub feed: Option<OptionsFeed>,
    pub r#type: Option<ContractType>,
    pub strike_price_gte: Option<Decimal>,
    pub strike_price_lte: Option<Decimal>,
    pub expiration_date: Option<String>,
    pub expiration_date_gte: Option<String>,
    pub expiration_date_lte: Option<String>,
    pub root_symbol: Option<String>,
    pub updated_since: Option<String>,
    pub limit: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ConditionCodesRequest {
    pub ticktype: TickType,
}

impl BarsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_option_symbols(&self.symbols, Some(MAX_OPTION_SYMBOLS_PER_REQUEST))?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_contract_symbols(&self.symbols));
        query.push_opt("timeframe", Some(self.timeframe));
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
        validate_option_symbols(&self.symbols, Some(MAX_OPTION_SYMBOLS_PER_REQUEST))?;
        validate_limit(self.limit, 1, 10_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_contract_symbols(&self.symbols));
        query.push_opt("start", self.start);
        query.push_opt("end", self.end);
        query.push_opt("limit", self.limit);
        query.push_opt("sort", self.sort);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }
}

impl LatestQuotesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_option_symbols(&self.symbols, Some(MAX_OPTION_SYMBOLS_PER_REQUEST))
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_query(&self.symbols, self.feed)
    }
}

impl LatestTradesRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_option_symbols(&self.symbols, Some(MAX_OPTION_SYMBOLS_PER_REQUEST))
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        latest_query(&self.symbols, self.feed)
    }
}

impl SnapshotsRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_option_symbols(&self.symbols, Some(MAX_OPTION_SYMBOLS_PER_REQUEST))?;
        validate_limit(self.limit, 1, 1_000)
    }

    pub(crate) fn validate_all(&self) -> Result<(), Error> {
        validate_option_symbols(&self.symbols, None)?;
        validate_limit(self.limit, 1, 1_000)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_csv("symbols", normalized_contract_symbols(&self.symbols));
        query.push_opt("feed", self.feed);
        query.push_opt("limit", self.limit);
        query.push_opt("page_token", self.page_token);
        query.finish()
    }

    pub(crate) fn batches(&self, max_symbols: usize) -> Vec<Self> {
        let normalized = normalized_contract_symbols(&self.symbols);
        if normalized.is_empty() {
            return Vec::new();
        }

        normalized
            .chunks(max_symbols)
            .map(|symbols| Self {
                symbols: symbols.to_vec(),
                feed: self.feed,
                limit: self.limit,
                page_token: None,
            })
            .collect()
    }
}

impl ChainRequest {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        validate_required_symbol(&self.underlying_symbol, "underlying_symbol")?;
        validate_limit(self.limit, 1, 1_000)
    }

    pub(crate) fn path_symbol(&self) -> String {
        options_underlying_symbol(&self.underlying_symbol)
    }

    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("feed", self.feed);
        query.push_opt("type", self.r#type);
        query.push_opt("strike_price_gte", self.strike_price_gte);
        query.push_opt("strike_price_lte", self.strike_price_lte);
        query.push_opt("expiration_date", self.expiration_date);
        query.push_opt("expiration_date_gte", self.expiration_date_gte);
        query.push_opt("expiration_date_lte", self.expiration_date_lte);
        query.push_opt(
            "root_symbol",
            self.root_symbol
                .map(|value| options_underlying_symbol(&value)),
        );
        query.push_opt("updated_since", self.updated_since);
        query.push_opt("limit", self.limit);
        query.push_opt("page_token", self.page_token);
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

impl PaginatedRequest for TradesRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

impl PaginatedRequest for SnapshotsRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

impl PaginatedRequest for ChainRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

fn latest_query(symbols: &[String], feed: Option<OptionsFeed>) -> Vec<(String, String)> {
    let mut query = QueryWriter::default();
    query.push_csv("symbols", normalized_contract_symbols(symbols));
    query.push_opt("feed", feed);
    query.finish()
}

fn validate_required_symbol(symbol: &str, field_name: &str) -> Result<(), Error> {
    if options_underlying_symbol(symbol).is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{field_name} is invalid: must not be empty or whitespace-only"
        )));
    }

    Ok(())
}

fn validate_option_symbols(symbols: &[String], max_symbols: Option<usize>) -> Result<(), Error> {
    if symbols.is_empty() {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not be empty".to_owned(),
        ));
    }

    if let Some(max_symbols) = max_symbols
        && symbols.len() > max_symbols
    {
        return Err(Error::InvalidRequest(format!(
            "symbols must contain at most {max_symbols} contract symbols"
        )));
    }

    if normalized_contract_symbols(symbols)
        .iter()
        .any(|symbol| symbol.is_empty())
    {
        return Err(Error::InvalidRequest(
            "symbols are invalid: must not contain empty or whitespace-only entries".to_owned(),
        ));
    }

    Ok(())
}

fn normalized_contract_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .map(|symbol| option_contract_symbol(symbol))
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
