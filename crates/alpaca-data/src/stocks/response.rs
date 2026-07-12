use std::collections::HashMap;

use alpaca_core::{Error, pagination::PaginatedResponse};
use serde::{Deserialize, Serialize};

use super::{Bar, Currency, DailyAuction, Quote, Snapshot, Trade};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Vec<Bar>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BarSingleResponse {
    pub symbol: String,
    #[serde(default)]
    pub bars: Vec<Bar>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

impl From<BarSingleResponse> for BarsResponse {
    fn from(response: BarSingleResponse) -> Self {
        Self {
            bars: HashMap::from([(response.symbol, response.bars)]),
            next_page_token: response.next_page_token,
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuctionsResponse {
    #[serde(default)]
    pub auctions: HashMap<String, Vec<DailyAuction>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AuctionSingleResponse {
    pub symbol: String,
    pub auctions: Option<Vec<DailyAuction>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

impl From<AuctionSingleResponse> for AuctionsResponse {
    fn from(response: AuctionSingleResponse) -> Self {
        Self {
            auctions: HashMap::from([(response.symbol, response.auctions.unwrap_or_default())]),
            next_page_token: response.next_page_token,
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Vec<Quote>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct QuoteSingleResponse {
    pub symbol: String,
    pub quotes: Option<Vec<Quote>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

impl From<QuoteSingleResponse> for QuotesResponse {
    fn from(response: QuoteSingleResponse) -> Self {
        Self {
            quotes: HashMap::from([(response.symbol, response.quotes.unwrap_or_default())]),
            next_page_token: response.next_page_token,
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Vec<Trade>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TradeSingleResponse {
    pub symbol: String,
    pub trades: Option<Vec<Trade>>,
    pub next_page_token: Option<String>,
    pub currency: Option<Currency>,
}

impl From<TradeSingleResponse> for TradesResponse {
    fn from(response: TradeSingleResponse) -> Self {
        Self {
            trades: HashMap::from([(response.symbol, response.trades.unwrap_or_default())]),
            next_page_token: response.next_page_token,
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestBarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Bar>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LatestBarSingleResponse {
    pub symbol: String,
    pub bar: Bar,
    pub currency: Option<Currency>,
}

impl From<LatestBarSingleResponse> for LatestBarsResponse {
    fn from(response: LatestBarSingleResponse) -> Self {
        Self {
            bars: HashMap::from([(response.symbol, response.bar)]),
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestQuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Quote>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LatestQuoteSingleResponse {
    pub symbol: String,
    pub quote: Quote,
    pub currency: Option<Currency>,
}

impl From<LatestQuoteSingleResponse> for LatestQuotesResponse {
    fn from(response: LatestQuoteSingleResponse) -> Self {
        Self {
            quotes: HashMap::from([(response.symbol, response.quote)]),
            currency: response.currency,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestTradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Trade>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LatestTradeSingleResponse {
    pub symbol: String,
    pub trade: Trade,
    pub currency: Option<Currency>,
}

impl From<LatestTradeSingleResponse> for LatestTradesResponse {
    fn from(response: LatestTradeSingleResponse) -> Self {
        Self {
            trades: HashMap::from([(response.symbol, response.trade)]),
            currency: response.currency,
        }
    }
}

pub type SnapshotsResponse = HashMap<String, Snapshot>;

#[derive(Debug, Deserialize)]
pub(super) struct SnapshotSingleResponse {
    pub symbol: String,
    #[serde(rename = "currency")]
    pub _currency: Option<Currency>,
    #[serde(flatten)]
    pub snapshot: Snapshot,
}

impl From<SnapshotSingleResponse> for SnapshotsResponse {
    fn from(response: SnapshotSingleResponse) -> Self {
        let SnapshotSingleResponse {
            symbol, snapshot, ..
        } = response;
        HashMap::from([(symbol, snapshot)])
    }
}

pub type ConditionCodesResponse = HashMap<String, String>;
pub type ExchangeCodesResponse = HashMap<String, String>;

impl PaginatedResponse for BarsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
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

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
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

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
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

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
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
) -> Result<(), Error> {
    match (currency.as_ref(), next_currency) {
        (Some(current), Some(next)) if current != &next => Err(Error::InvalidRequest(format!(
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
