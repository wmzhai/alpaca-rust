use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::cache::state::{
    BarsMap, CacheState, StockBarsRequest, collect_cached_hits, normalize_option_symbols,
    normalize_stock_symbols,
};
use crate::cache::stats::CacheStats;
use crate::options::{self, OptionsFeed, SnapshotsRequest as OptionSnapshotsRequest};
use crate::stocks::{self, DataFeed, SnapshotsRequest as StockSnapshotsRequest};
use crate::{Client, Error};

#[derive(Clone)]
pub struct CachedClientConfig {
    pub stocks_feed: Arc<dyn Fn() -> DataFeed + Send + Sync>,
    pub options_feed: OptionsFeed,
}

impl Default for CachedClientConfig {
    fn default() -> Self {
        Self {
            stocks_feed: Arc::new(|| stocks::preferred_feed(false)),
            options_feed: options::preferred_feed(),
        }
    }
}

pub struct CachedClient {
    raw: Client,
    config: CachedClientConfig,
    state: RwLock<CacheState>,
}

impl CachedClient {
    #[must_use]
    pub fn new(raw: Client) -> Self {
        Self::with_config(raw, CachedClientConfig::default())
    }

    #[must_use]
    pub fn with_config(raw: Client, config: CachedClientConfig) -> Self {
        Self {
            raw,
            config,
            state: RwLock::new(CacheState::default()),
        }
    }

    #[must_use]
    pub fn raw(&self) -> &Client {
        &self.raw
    }

    pub async fn stocks<S: AsRef<str>>(
        &self,
        symbols: &[S],
    ) -> Result<HashMap<String, stocks::Snapshot>, Error> {
        let requested = normalize_stock_symbols(symbols);
        if requested.is_empty() {
            return Ok(HashMap::new());
        }

        let resolved = unique_resolved_symbols(&requested);
        let (mut hits, missing) = {
            let state = self.state.read().await;
            collect_cached_hits(&resolved, &state.stocks.values, &state.stocks.empty)
        };

        if !missing.is_empty() {
            let fetched = self.fetch_stocks(&missing).await?;
            let mut state = self.state.write().await;
            for symbol in &missing {
                state.stocks.subscribed.insert(symbol.clone());
                if fetched.contains_key(symbol) {
                    state.stocks.empty.remove(symbol);
                } else {
                    state.stocks.empty.insert(symbol.clone());
                }
            }
            for (symbol, snapshot) in &fetched {
                state.stocks.values.insert(symbol.clone(), snapshot.clone());
            }
            state.stocks.updated_at = Some(SystemTime::now());
            hits.extend(fetched);
        }

        Ok(requested
            .into_iter()
            .filter_map(|(original, resolved)| {
                hits.get(&resolved)
                    .cloned()
                    .map(|snapshot| (original, snapshot))
            })
            .collect())
    }

    pub async fn stock(&self, symbol: &str) -> Option<stocks::Snapshot> {
        self.stocks(&[symbol])
            .await
            .ok()?
            .into_iter()
            .next()
            .map(|(_, snapshot)| snapshot)
    }

    pub async fn options<S: AsRef<str>>(
        &self,
        contracts: &[S],
    ) -> Result<HashMap<String, options::Snapshot>, Error> {
        let requested = normalize_option_symbols(contracts);
        if requested.is_empty() {
            return Ok(HashMap::new());
        }

        let (mut hits, missing) = {
            let state = self.state.read().await;
            collect_cached_hits(&requested, &state.options.values, &state.options.empty)
        };

        if !missing.is_empty() {
            let fetched = self.fetch_options(&missing).await?;
            let mut state = self.state.write().await;
            for contract in &missing {
                state.options.subscribed.insert(contract.clone());
                if fetched.contains_key(contract) {
                    state.options.empty.remove(contract);
                } else {
                    state.options.empty.insert(contract.clone());
                }
            }
            for (contract, snapshot) in &fetched {
                state.options.values.insert(contract.clone(), snapshot.clone());
            }
            state.options.updated_at = Some(SystemTime::now());
            hits.extend(fetched);
        }

        Ok(requested
            .into_iter()
            .filter_map(|contract| hits.remove_entry(&contract))
            .collect())
    }

    pub async fn option(&self, contract: &str) -> Option<options::Snapshot> {
        self.options(&[contract]).await.ok()?.remove(contract)
    }

    pub async fn watch_stocks(&self, symbols: &[String]) {
        let normalized = normalize_stock_symbols(symbols);
        let mut state = self.state.write().await;
        for (_, symbol) in normalized {
            state.stocks.subscribed.insert(symbol);
        }
    }

    pub async fn watch_options(&self, contracts: &[String]) {
        let normalized = normalize_option_symbols(contracts);
        let mut state = self.state.write().await;
        for contract in normalized {
            state.options.subscribed.insert(contract);
        }
    }

    pub async fn refresh_stocks(&self) -> Result<usize, Error> {
        let symbols = {
            let state = self.state.read().await;
            state.stocks.subscribed.iter().cloned().collect::<Vec<_>>()
        };
        if symbols.is_empty() {
            return Ok(0);
        }

        let fetched = self.fetch_stocks(&symbols).await?;
        let count = fetched.len();

        let mut state = self.state.write().await;
        for (symbol, snapshot) in fetched {
            state.stocks.values.insert(symbol, snapshot);
        }
        state.stocks.updated_at = Some(SystemTime::now());
        Ok(count)
    }

    pub async fn refresh_options(&self) -> Result<usize, Error> {
        let contracts = {
            let state = self.state.read().await;
            state.options.subscribed.iter().cloned().collect::<Vec<_>>()
        };
        if contracts.is_empty() {
            return Ok(0);
        }

        let fetched = self.fetch_options(&contracts).await?;
        let count = fetched.len();

        let mut state = self.state.write().await;
        for (contract, snapshot) in fetched {
            state.options.values.insert(contract, snapshot);
        }
        state.options.updated_at = Some(SystemTime::now());
        Ok(count)
    }

    pub async fn watch_bars(&self, request: StockBarsRequest) {
        let request = request.normalized();
        let mut state = self.state.write().await;
        state
            .bars
            .requests
            .entry(request.key.clone())
            .and_modify(|current| current.merge_from(&request))
            .or_insert(request);
    }

    pub async fn bars(&self, key: &str) -> Result<HashMap<String, Vec<stocks::BarPoint>>, Error> {
        let request = self.bars_request(key).await?;
        let missing = {
            let state = self.state.read().await;
            let cached = state.bars.values.get(key);
            let empty = state.bars.empty.get(key);

            request
                .symbols
                .iter()
                .filter(|symbol| {
                    !cached.is_some_and(|bars| bars.contains_key(*symbol))
                        && !empty.is_some_and(|values| values.contains(*symbol))
                })
                .cloned()
                .collect::<Vec<_>>()
        };

        if missing.is_empty() {
            let state = self.state.read().await;
            return Ok(state.bars.values.get(key).cloned().unwrap_or_default());
        }

        self.fetch_missing_bars(key, &request, &missing).await
    }

    pub async fn bar(&self, key: &str, symbol: &str) -> Option<Vec<stocks::BarPoint>> {
        let resolved = stocks::display_stock_symbol(symbol);
        {
            let state = self.state.read().await;
            if let Some(values) = state.bars.values.get(key)
                && let Some(bars) = values.get(&resolved)
            {
                return Some(bars.clone());
            }
            if state
                .bars
                .empty
                .get(key)
                .is_some_and(|symbols| symbols.contains(&resolved))
            {
                return None;
            }
        }

        self.bars(key)
            .await
            .ok()?
            .get(&resolved)
            .cloned()
    }

    pub async fn refresh_bars(&self, key: &str) -> Result<usize, Error> {
        let request = self.bars_request(key).await?;
        let fetched = self.fetch_bars_request(&request, &request.symbols).await?;
        let count = fetched.len();

        let missing: HashSet<String> = request
            .symbols
            .iter()
            .filter(|symbol| !fetched.contains_key(*symbol))
            .cloned()
            .collect();

        let mut state = self.state.write().await;
        state.bars.values.insert(key.to_string(), fetched);
        state.bars.empty.insert(key.to_string(), missing);
        state.bars.updated_at.insert(key.to_string(), SystemTime::now());
        Ok(count)
    }

    pub async fn clear_options(&self) {
        let mut state = self.state.write().await;
        state.options.subscribed.clear();
        state.options.values.clear();
        state.options.empty.clear();
        state.options.updated_at = None;
    }

    pub async fn stats(&self) -> CacheStats {
        let state = self.state.read().await;
        CacheStats {
            subscribed_symbols: state.stocks.subscribed.len(),
            subscribed_contracts: state.options.subscribed.len(),
            subscribed_bar_requests: state.bars.requests.len(),
            cached_stocks: state.stocks.values.len(),
            cached_options: state.options.values.len(),
            cached_bar_symbols: state.bars.values.values().map(HashMap::len).sum(),
            stocks_updated_at: format_timestamp(state.stocks.updated_at),
            options_updated_at: format_timestamp(state.options.updated_at),
            bars_updated_at: state
                .bars
                .updated_at
                .iter()
                .map(|(key, value)| (key.clone(), format_timestamp(Some(*value)).unwrap_or_default()))
                .collect(),
        }
    }

    async fn fetch_stocks(&self, symbols: &[String]) -> Result<HashMap<String, stocks::Snapshot>, Error> {
        self.raw
            .stocks()
            .snapshots(StockSnapshotsRequest {
                symbols: symbols.to_vec(),
                feed: Some((self.config.stocks_feed)()),
                currency: None,
            })
            .await
    }

    async fn fetch_options(
        &self,
        contracts: &[String],
    ) -> Result<HashMap<String, options::Snapshot>, Error> {
        self.raw
            .options()
            .snapshots_all(OptionSnapshotsRequest {
                symbols: contracts.to_vec(),
                feed: Some(self.config.options_feed),
                limit: Some(1000),
                page_token: None,
            })
            .await
            .map(|response| response.snapshots)
    }

    async fn bars_request(&self, key: &str) -> Result<StockBarsRequest, Error> {
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::InvalidRequest(
                "bars key is invalid: must not be empty".to_owned(),
            ));
        }

        let state = self.state.read().await;
        state.bars.requests.get(key).cloned().ok_or_else(|| {
            Error::InvalidRequest(format!("bars key is unknown: {key}"))
        })
    }

    async fn fetch_missing_bars(
        &self,
        key: &str,
        request: &StockBarsRequest,
        missing: &[String],
    ) -> Result<HashMap<String, Vec<stocks::BarPoint>>, Error> {
        let fetched = self.fetch_bars_request(request, missing).await?;
        let missing_empty: HashSet<String> = missing
            .iter()
            .filter(|symbol| !fetched.contains_key(*symbol))
            .cloned()
            .collect();

        let mut state = self.state.write().await;
        let key = key.to_string();
        {
            let cached = state.bars.values.entry(key.clone()).or_default();
            for (symbol, bars) in &fetched {
                cached.insert(symbol.clone(), bars.clone());
            }
        }
        {
            let empty = state.bars.empty.entry(key.clone()).or_default();
            for symbol in missing {
                if missing_empty.contains(symbol) {
                    empty.insert(symbol.clone());
                } else {
                    empty.remove(symbol);
                }
            }
        }
        state.bars.updated_at.insert(key.clone(), SystemTime::now());

        Ok(state.bars.values.get(&key).cloned().unwrap_or_default())
    }

    async fn fetch_bars_request(
        &self,
        request: &StockBarsRequest,
        symbols: &[String],
    ) -> Result<BarsMap, Error> {
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        let mut merged = HashMap::new();
        let chunk_size = request.chunk_size.max(1);
        let daily = request.timeframe == stocks::TimeFrame::day_1();

        for chunk in symbols.chunks(chunk_size) {
            let response = self
                .raw
                .stocks()
                .bars_all(stocks::BarsRequest {
                    symbols: chunk.to_vec(),
                    timeframe: request.timeframe.clone(),
                    start: request.start.clone(),
                    end: request.end.clone(),
                    limit: Some(request.limit),
                    adjustment: request.adjustment.clone(),
                    feed: request.feed,
                    sort: None,
                    asof: None,
                    currency: request.currency.clone(),
                    page_token: None,
                })
                .await?;

            for (symbol, bars) in response.bars {
                merged.insert(
                    symbol,
                    bars.into_iter().map(|bar| bar.point(daily)).collect(),
                );
            }
        }

        Ok(merged)
    }
}

fn unique_resolved_symbols(requested: &[(String, String)]) -> Vec<String> {
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    for (_, symbol) in requested {
        if seen.insert(symbol.clone()) {
            resolved.push(symbol.clone());
        }
    }
    resolved
}

fn format_timestamp(value: Option<SystemTime>) -> Option<String> {
    value.map(|value| {
        DateTime::<Utc>::from(value)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    })
}
