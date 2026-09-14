use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime};

use crate::options;
use crate::stocks::{self, Adjustment, BarPoint, Currency, DataFeed, TimeFrame};
use crate::symbols::option_contract_symbol;

pub(crate) type BarsMap = HashMap<String, Vec<BarPoint>>;

#[derive(Debug, Clone)]
pub struct CachedEntry<T> {
    pub value: T,
    pub stored_at: Instant,
}

impl<T> CachedEntry<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            stored_at: Instant::now(),
        }
    }

    pub fn is_fresh(&self, ttl: Duration, now: Instant) -> bool {
        is_timestamp_fresh(self.stored_at, ttl, now)
    }
}

pub fn is_timestamp_fresh(stored_at: Instant, ttl: Duration, now: Instant) -> bool {
    now.saturating_duration_since(stored_at) < ttl
}

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
    pub values: HashMap<String, CachedEntry<T>>,
    pub empty: HashMap<String, Instant>,
    pub updated_at: Option<SystemTime>,
}

impl<T: Clone> SnapshotCache<T> {
    pub(crate) fn reconcile(
        &mut self,
        requested: &[String],
        fetched: &HashMap<String, T>,
        updated_at: SystemTime,
    ) -> usize {
        let mut count = 0;
        let mut seen = HashSet::new();
        let stored_at = Instant::now();

        for key in requested {
            if !seen.insert(key) {
                continue;
            }

            self.subscribed.insert(key.clone());
            if let Some(value) = fetched.get(key) {
                self.values.insert(
                    key.clone(),
                    CachedEntry {
                        value: value.clone(),
                        stored_at,
                    },
                );
                self.empty.remove(key);
                count += 1;
            } else {
                self.values.remove(key);
                self.empty.insert(key.clone(), stored_at);
            }
        }

        if !seen.is_empty() {
            self.updated_at = Some(updated_at);
        }
        count
    }
}

#[derive(Debug, Default)]
pub(crate) struct StockBarsCache {
    pub requests: HashMap<String, StockBarsRequest>,
    pub values: HashMap<String, HashMap<String, CachedEntry<Vec<BarPoint>>>>,
    pub empty: HashMap<String, HashMap<String, Instant>>,
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

pub fn collect_cached_hits<T: Clone>(
    requested: &[String],
    cached: &HashMap<String, CachedEntry<T>>,
    empty: &HashMap<String, Instant>,
    ttl: Duration,
    now: Instant,
) -> (HashMap<String, T>, Vec<String>) {
    let mut hits = HashMap::new();
    let mut missing = Vec::new();
    for key in requested {
        if let Some(entry) = cached.get(key) {
            if entry.is_fresh(ttl, now) {
                hits.insert(key.clone(), entry.value.clone());
            } else {
                missing.push(key.clone());
            }
        } else if empty
            .get(key)
            .is_some_and(|stored_at| is_timestamp_fresh(*stored_at, ttl, now))
        {
            continue;
        } else {
            missing.push(key.clone());
        }
    }
    (hits, missing)
}

pub(crate) fn unwrap_bars_map(
    cached: &HashMap<String, CachedEntry<Vec<BarPoint>>>,
) -> BarsMap {
    cached
        .iter()
        .map(|(symbol, entry)| (symbol.clone(), entry.value.clone()))
        .collect()
}

pub(crate) fn missing_bar_symbols(
    requested: &[String],
    cached: Option<&HashMap<String, CachedEntry<Vec<BarPoint>>>>,
    empty: Option<&HashMap<String, Instant>>,
    ttl: Duration,
    now: Instant,
) -> Vec<String> {
    requested
        .iter()
        .filter(|symbol| {
            if cached
                .and_then(|values| values.get(*symbol))
                .is_some_and(|entry| entry.is_fresh(ttl, now))
            {
                return false;
            }
            if empty
                .and_then(|values| values.get(*symbol))
                .is_some_and(|stored_at| is_timestamp_fresh(*stored_at, ttl, now))
            {
                return false;
            }
            true
        })
        .cloned()
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_price_entries_are_treated_as_missing() {
        let now = Instant::now();
        let stale = now
            .checked_sub(Duration::from_secs(20))
            .expect("ttl test clock");
        let mut cached = HashMap::new();
        cached.insert(
            "AAPL".to_string(),
            CachedEntry {
                value: 1,
                stored_at: stale,
            },
        );
        cached.insert(
            "MSFT".to_string(),
            CachedEntry {
                value: 2,
                stored_at: now,
            },
        );

        let (hits, missing) = collect_cached_hits(
            &[
                "AAPL".to_string(),
                "MSFT".to_string(),
                "GOOG".to_string(),
            ],
            &cached,
            &HashMap::new(),
            Duration::from_secs(15),
            now,
        );

        assert_eq!(hits.get("MSFT"), Some(&2));
        assert!(!hits.contains_key("AAPL"));
        assert_eq!(missing, vec!["AAPL".to_string(), "GOOG".to_string()]);
    }

    #[test]
    fn expired_empty_entries_are_retried() {
        let now = Instant::now();
        let stale = now
            .checked_sub(Duration::from_secs(20))
            .expect("ttl test clock");
        let mut empty = HashMap::new();
        empty.insert("AAPL".to_string(), stale);
        empty.insert("MSFT".to_string(), now);

        let (hits, missing) = collect_cached_hits(
            &["AAPL".to_string(), "MSFT".to_string()],
            &HashMap::<String, CachedEntry<i32>>::new(),
            &empty,
            Duration::from_secs(15),
            now,
        );

        assert!(hits.is_empty());
        assert_eq!(missing, vec!["AAPL".to_string()]);
    }
}
