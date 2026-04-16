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
pub struct AuctionsResponse {
    #[serde(default)]
    pub auctions: HashMap<String, Vec<DailyAuction>>,
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
pub struct TradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Vec<Trade>>,
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
pub struct LatestQuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Quote>,
    pub currency: Option<Currency>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestTradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Trade>,
    pub currency: Option<Currency>,
}

pub type SnapshotsResponse = HashMap<String, Snapshot>;

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

fn merge_batch_currency(
    operation: &str,
    currency: &mut Option<Currency>,
    next_currency: Option<Currency>,
) -> Result<(), CoreError> {
    match (currency.as_ref(), next_currency) {
        (Some(current), Some(next)) if current != &next => Err(CoreError::InvalidRequest(format!(
            "{operation} received mismatched currency across pages: expected {}, got {}",
            current.as_str(),
            next.as_str()
        ))),
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
