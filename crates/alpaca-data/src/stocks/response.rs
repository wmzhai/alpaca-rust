use std::collections::HashMap;

use alpaca_core::{Error as CoreError, pagination::PaginatedResponse};
use serde::{Deserialize, Serialize};

use super::{Bar, Currency, DailyAuction, Quote, Snapshot, Trade};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Vec<Bar>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarsSingleResponse {
    pub symbol: String,
    #[serde(default)]
    pub bars: Vec<Bar>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuctionsResponse {
    #[serde(default)]
    pub auctions: HashMap<String, Vec<DailyAuction>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuctionsSingleResponse {
    pub symbol: String,
    #[serde(default)]
    pub auctions: Vec<DailyAuction>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Vec<Quote>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuotesSingleResponse {
    pub symbol: String,
    #[serde(default)]
    pub quotes: Vec<Quote>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Vec<Trade>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradesSingleResponse {
    pub symbol: String,
    #[serde(default)]
    pub trades: Vec<Trade>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestBarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Bar>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestBarResponse {
    pub symbol: String,
    pub bar: Bar,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestQuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Quote>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestQuoteResponse {
    pub symbol: String,
    pub quote: Quote,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestTradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Trade>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestTradeResponse {
    pub symbol: String,
    pub trade: Trade,
    pub currency: Option<Currency>,
}

pub type SnapshotsResponse = HashMap<String, Snapshot>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub symbol: String,
    pub currency: Option<Currency>,
    #[serde(rename = "latestTrade")]
    pub latest_trade: Option<Trade>,
    #[serde(rename = "latestQuote")]
    pub latest_quote: Option<Quote>,
    #[serde(rename = "minuteBar")]
    pub minute_bar: Option<Bar>,
    #[serde(rename = "dailyBar")]
    pub daily_bar: Option<Bar>,
    #[serde(rename = "prevDailyBar")]
    pub prev_daily_bar: Option<Bar>,
}

pub type ConditionCodesResponse = HashMap<String, String>;
pub type ExchangeCodesResponse = HashMap<String, String>;

impl PaginatedResponse for BarsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_batch_currency("stocks.bars_all", &mut self.currency, next.currency)?;
        merge_batch_page(&mut self.bars, next.bars);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for BarsSingleResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_single_metadata(
            "stocks.bars_single_all",
            &mut self.symbol,
            &mut self.currency,
            next.symbol,
            next.currency,
        )?;
        self.bars.extend(next.bars);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for AuctionsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_batch_currency("stocks.auctions_all", &mut self.currency, next.currency)?;
        merge_batch_page(&mut self.auctions, next.auctions);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for AuctionsSingleResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_single_metadata(
            "stocks.auctions_single_all",
            &mut self.symbol,
            &mut self.currency,
            next.symbol,
            next.currency,
        )?;
        self.auctions.extend(next.auctions);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for QuotesResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_batch_currency("stocks.quotes_all", &mut self.currency, next.currency)?;
        merge_batch_page(&mut self.quotes, next.quotes);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for QuotesSingleResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_single_metadata(
            "stocks.quotes_single_all",
            &mut self.symbol,
            &mut self.currency,
            next.symbol,
            next.currency,
        )?;
        self.quotes.extend(next.quotes);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for TradesResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_batch_currency("stocks.trades_all", &mut self.currency, next.currency)?;
        merge_batch_page(&mut self.trades, next.trades);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for TradesSingleResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_single_metadata(
            "stocks.trades_single_all",
            &mut self.symbol,
            &mut self.currency,
            next.symbol,
            next.currency,
        )?;
        self.trades.extend(next.trades);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

fn merge_batch_currency(
    operation: &str,
    currency: &mut Option<Currency>,
    next_currency: Option<Currency>,
) -> Result<(), CoreError> {
    match (currency.as_ref(), next_currency) {
        (Some(current), Some(next)) if current != &next => Err(CoreError::InvalidRequest(
            format!(
                "{operation} received mismatched currency across pages: expected {}, got {}",
                current.as_str(),
                next.as_str()
            ),
        )),
        (None, Some(next)) => {
            *currency = Some(next);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn merge_batch_page<Item>(
    current: &mut HashMap<String, Vec<Item>>,
    next: HashMap<String, Vec<Item>>,
) {
    for (symbol, mut items) in next {
        current.entry(symbol).or_default().append(&mut items);
    }
}

fn merge_single_metadata(
    operation: &str,
    symbol: &mut String,
    currency: &mut Option<Currency>,
    next_symbol: String,
    next_currency: Option<Currency>,
) -> Result<(), CoreError> {
    if !symbol.is_empty() && *symbol != next_symbol {
        return Err(CoreError::InvalidRequest(format!(
            "{operation} received mismatched symbol across pages: expected {}, got {}",
            symbol, next_symbol
        )));
    }

    if symbol.is_empty() {
        *symbol = next_symbol;
    }

    match (currency.as_ref(), next_currency) {
        (Some(current), Some(next)) if current != &next => Err(CoreError::InvalidRequest(
            format!(
                "{operation} received mismatched currency across pages: expected {}, got {}",
                current.as_str(),
                next.as_str()
            ),
        )),
        (None, Some(next)) => {
            *currency = Some(next);
            Ok(())
        }
        _ => Ok(()),
    }
}
