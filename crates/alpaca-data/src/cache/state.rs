use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use crate::options;
use crate::stocks::{self, Adjustment, BarPoint, Currency, DataFeed, TimeFrame};
use crate::symbols::option_contract_symbol;

pub(crate) type BarsMap = HashMap<String, Vec<BarPoint>>;

#[derive(Debug, Clone)]
pub struct StockBarsRequest {
    pub key: String,
    pub symbols: Vec<String>,
    pub timeframe: TimeFrame,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: u32,
    pub adjustment: Option<Adjustment>,
    pub feed: Option<DataFeed>,
    pub currency: Option<Currency>,
    pub chunk_size: usize,
}

impl StockBarsRequest {
    pub(crate) fn normalized(mut self) -> Self {
        self.symbols = normalize_stock_list(&self.symbols);
        self
    }

    pub(crate) fn merge_from(&mut self, next: &Self) {
        self.symbols = merge_values(&self.symbols, &next.symbols);
        self.timeframe = next.timeframe.clone();
        self.start = next.start.clone();
        self.end = next.end.clone();
        self.limit = next.limit;
        self.adjustment = next.adjustment.clone();
        self.feed = next.feed;
        self.currency = next.currency.clone();
        self.chunk_size = next.chunk_size;
    }
}

#[derive(Debug, Default)]
pub(crate) struct SnapshotCache<T> {
    pub subscribed: HashSet<String>,
    pub values: HashMap<String, T>,
    pub empty: HashSet<String>,
    pub updated_at: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub(crate) struct StockBarsCache {
    pub requests: HashMap<String, StockBarsRequest>,
    pub values: HashMap<String, BarsMap>,
    pub empty: HashMap<String, HashSet<String>>,
    pub updated_at: HashMap<String, SystemTime>,
}

#[derive(Debug, Default)]
pub(crate) struct CacheState {
    pub stocks: SnapshotCache<stocks::Snapshot>,
    pub options: SnapshotCache<options::Snapshot>,
    pub bars: StockBarsCache,
}

pub(crate) fn normalize_values<S: AsRef<str>>(values: &[S]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let value = value.as_ref().trim();
        if !value.is_empty() && seen.insert(value.to_string()) {
            normalized.push(value.to_string());
        }
    }
    normalized
}

pub(crate) fn normalize_stock_symbols<S: AsRef<str>>(symbols: &[S]) -> Vec<(String, String)> {
    normalize_values(symbols)
        .into_iter()
        .map(|symbol| {
            let resolved = stocks::display_stock_symbol(&symbol);
            (symbol, resolved)
        })
        .collect()
}

pub(crate) fn normalize_option_symbols<S: AsRef<str>>(symbols: &[S]) -> Vec<String> {
    let normalized = normalize_values(symbols);
    let mut values = Vec::new();
    let mut seen = HashSet::new();
    for symbol in normalized {
        let symbol = option_contract_symbol(&symbol);
        if !symbol.is_empty() && seen.insert(symbol.clone()) {
            values.push(symbol);
        }
    }
    values
}

pub(crate) fn collect_cached_hits<T: Clone>(
    requested: &[String],
    cached: &HashMap<String, T>,
    empty: &HashSet<String>,
) -> (HashMap<String, T>, Vec<String>) {
    let mut hits = HashMap::new();
    let mut missing = Vec::new();
    for key in requested {
        if let Some(value) = cached.get(key) {
            hits.insert(key.clone(), value.clone());
        } else if !empty.contains(key) {
            missing.push(key.clone());
        }
    }
    (hits, missing)
}

fn normalize_stock_list<S: AsRef<str>>(symbols: &[S]) -> Vec<String> {
    normalize_stock_symbols(symbols)
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect()
}

fn merge_values(current: &[String], next: &[String]) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    for value in current.iter().chain(next.iter()) {
        if seen.insert(value.clone()) {
            merged.push(value.clone());
        }
    }
    merged
}
