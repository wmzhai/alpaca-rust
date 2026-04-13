use std::collections::HashMap;

use alpaca_core::{Error as CoreError, pagination::PaginatedResponse};
use serde::{Deserialize, Serialize};

use super::{Bar, Quote, Snapshot, Trade};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarsResponse {
    #[serde(default)]
    pub bars: HashMap<String, Vec<Bar>>,
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TradesResponse {
    #[serde(default)]
    pub trades: HashMap<String, Vec<Trade>>,
    pub next_page_token: Option<String>,
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
pub struct SnapshotsResponse {
    #[serde(default)]
    pub snapshots: HashMap<String, Snapshot>,
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChainResponse {
    #[serde(default)]
    pub snapshots: HashMap<String, Snapshot>,
    pub next_page_token: Option<String>,
}

pub type ConditionCodesResponse = HashMap<String, String>;
pub type ExchangeCodesResponse = HashMap<String, String>;

impl PaginatedResponse for BarsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_batch_page(&mut self.bars, next.bars);
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
        merge_batch_page(&mut self.trades, next.trades);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for SnapshotsResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_snapshot_page("options.snapshots_all", &mut self.snapshots, next.snapshots)?;
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

impl PaginatedResponse for ChainResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        merge_snapshot_page("options.chain_all", &mut self.snapshots, next.snapshots)?;
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

pub(crate) fn merge_snapshot_page(
    operation: &str,
    current: &mut HashMap<String, Snapshot>,
    next: HashMap<String, Snapshot>,
) -> Result<(), CoreError> {
    for (symbol, snapshot) in next {
        if current.insert(symbol.clone(), snapshot).is_some() {
            return Err(CoreError::InvalidRequest(format!(
                "{operation} received duplicate symbol across pages: {symbol}"
            )));
        }
    }

    Ok(())
}
