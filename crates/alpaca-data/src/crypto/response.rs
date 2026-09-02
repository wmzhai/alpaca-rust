use std::collections::HashMap;

use alpaca_core::pagination::PaginatedResponse;
use serde::{Deserialize, Serialize};

use super::{Bar, Orderbook, Quote, Snapshot, Trade};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Vec<Bar>>,
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Vec<Quote>>,
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Vec<Trade>>,
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestBarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Bar>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestQuotesResponse {
    #[serde(default)]
    pub quotes: HashMap<String, Quote>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestTradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Trade>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LatestOrderbooksResponse {
    #[serde(default)]
    pub orderbooks: HashMap<String, Orderbook>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapshotsResponse {
    #[serde(default)]
    pub snapshots: HashMap<String, Snapshot>,
}

impl PaginatedResponse for BarsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), alpaca_core::Error> {
        merge_batch_page(&mut self.bars, next.bars);
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

    fn merge_page(&mut self, next: Self) -> Result<(), alpaca_core::Error> {
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

    fn merge_page(&mut self, next: Self) -> Result<(), alpaca_core::Error> {
        merge_batch_page(&mut self.trades, next.trades);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
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
