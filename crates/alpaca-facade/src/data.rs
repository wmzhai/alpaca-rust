use crate::{OptionChainRequest, map_live_snapshots, required_underlying_display_symbols};
use ::chrono::NaiveDateTime;
use alpaca_data::Client;
use alpaca_data::cache::{CacheStats as RawCacheStats, CachedClient, StockBarsRequest};
use alpaca_data::corporate_actions::{CorporateActionType, ListRequest};
use alpaca_data::stocks::{self, BarPoint, TimeFrame, preferred_feed as preferred_stock_feed};
use alpaca_option::contract;
use alpaca_option::url;
use alpaca_option::{OptionChain, OptionError, OptionPosition, OptionSnapshot, OrderSide};
use anyhow::{Context, Result};
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

use alpaca_time::{chrono, clock, range, session};

pub type BarsMap = HashMap<String, Vec<BarPoint>>;

/// Cache metadata for the facade-level enriched option and chain caches.
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub subscribed_symbols: usize,
    pub subscribed_contracts: usize,
    pub subscribed_chains: usize,
    pub subscribed_bar_requests: usize,
    pub cached_stocks: usize,
    pub cached_options: usize,
    pub cached_chains: usize,
    pub cached_bar_symbols: usize,
    pub stocks_updated_at: Option<String>,
    pub options_updated_at: Option<String>,
    pub contracts_updated_at: Option<String>,
    pub chains_updated_at: Option<String>,
    pub bars_updated_at: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AlpacaDataConfig {
    pub risk_free_rate: f64,
    pub dividend_yield: f64,
}

impl Default for AlpacaDataConfig {
    fn default() -> Self {
        Self {
            risk_free_rate: 0.0362,
            dividend_yield: 0.0,
        }
    }
}

#[derive(Default)]
struct OptionCache {
    subscribed: HashSet<String>,
    values: HashMap<String, OptionSnapshot>,
    empty: HashSet<String>,
    updated_at: Option<NaiveDateTime>,
}

#[derive(Default)]
struct ChainCache {
    subscribed: HashMap<String, OptionChainRequest>,
    values: HashMap<String, OptionChain>,
    cached_params: HashMap<String, OptionChainRequest>,
    updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Copy)]
enum BarsWindow {
    Day,
    Week,
    Month,
}

impl BarsWindow {
    fn key(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }

    fn timeframe(self) -> TimeFrame {
        match self {
            Self::Day => TimeFrame::day_1(),
            Self::Week => TimeFrame::from("1Week"),
            Self::Month => TimeFrame::from("1Month"),
        }
    }

    fn lookback_days(self) -> i32 {
        match self {
            Self::Day => -400,
            Self::Week => -2200,
            Self::Month => -3700,
        }
    }

    fn refresh_label(self) -> &'static str {
        match self {
            Self::Day => "day bars",
            Self::Week => "week bars",
            Self::Month => "month bars",
        }
    }
}

/// High-level Alpaca market-data facade built on top of `alpaca-data` raw
/// cache primitives.
pub struct AlpacaData {
    pub raw: CachedClient,
    config: AlpacaDataConfig,
    options: RwLock<OptionCache>,
    chains: RwLock<ChainCache>,
}

impl AlpacaData {
    #[must_use]
    pub fn with_raw(raw: CachedClient, config: AlpacaDataConfig) -> Self {
        Self {
            raw,
            config,
            options: RwLock::new(OptionCache::default()),
            chains: RwLock::new(ChainCache::default()),
        }
    }

    fn sdk(&self) -> &Client {
        self.raw.raw()
    }

    fn now_timestamp() -> NaiveDateTime {
        chrono::timestamp(None).expect("chrono::timestamp should always succeed for now()")
    }

    fn normalize_values<S: AsRef<str>>(values: &[S]) -> Vec<String> {
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

    fn normalize_stock_symbols<S: AsRef<str>>(symbols: &[S]) -> Vec<(String, String)> {
        Self::normalize_values(symbols)
            .into_iter()
            .map(|symbol| {
                let resolved = stocks::display_stock_symbol(&symbol);
                (symbol, resolved)
            })
            .collect()
    }

    fn normalize_option_symbol(contract_symbol: &str) -> Option<String> {
        let contract_symbol = contract_symbol.trim();
        if contract_symbol.is_empty() {
            return None;
        }

        Some(
            contract::parse_occ_symbol(contract_symbol)
                .map(|contract| contract.occ_symbol)
                .unwrap_or_else(|| contract_symbol.to_ascii_uppercase()),
        )
    }

    fn normalize_option_symbols<S: AsRef<str>>(contracts: &[S]) -> Vec<String> {
        let mut normalized = Vec::new();
        let mut seen = HashSet::new();
        for contract in contracts {
            let Some(contract) = Self::normalize_option_symbol(contract.as_ref()) else {
                continue;
            };
            if seen.insert(contract.clone()) {
                normalized.push(contract);
            }
        }
        normalized
    }

    fn collect_cached_hits<T: Clone>(
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

    fn option_pricing_inputs(&self) -> (f64, f64) {
        (self.config.risk_free_rate, self.config.dividend_yield)
    }

    fn bars_start(window: BarsWindow) -> String {
        range::add_days(&clock::today(), window.lookback_days()).unwrap_or_else(|_| clock::today())
    }

    fn format_datetime(value: Option<NaiveDateTime>) -> Option<String> {
        value.map(|datetime| datetime.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    fn bars_request(window: BarsWindow, symbols: &[String]) -> StockBarsRequest {
        StockBarsRequest {
            key: window.key().to_string(),
            symbols: symbols.to_vec(),
            timeframe: window.timeframe(),
            start: Some(Self::bars_start(window)),
            end: None,
            limit: 10_000,
            adjustment: None,
            feed: Some(preferred_stock_feed(session::is_overnight_window(
                &clock::now(),
            ))),
            currency: None,
            chunk_size: 25,
        }
    }

    pub fn day_bars_request(symbols: &[String]) -> StockBarsRequest {
        Self::bars_request(BarsWindow::Day, symbols)
    }

    pub fn week_bars_request(symbols: &[String]) -> StockBarsRequest {
        Self::bars_request(BarsWindow::Week, symbols)
    }

    pub fn month_bars_request(symbols: &[String]) -> StockBarsRequest {
        Self::bars_request(BarsWindow::Month, symbols)
    }

    pub async fn options<S: AsRef<str>>(
        &self,
        contracts: &[S],
    ) -> Result<HashMap<String, OptionSnapshot>> {
        self.ensure_options(contracts).await
    }

    pub async fn option(&self, contract: &str) -> Option<OptionSnapshot> {
        let contract = Self::normalize_option_symbol(contract)?;
        self.options(&[contract.as_str()])
            .await
            .ok()?
            .remove(&contract)
    }

    pub async fn resolve_optionstrat_url(
        &self,
        value: &str,
    ) -> Result<(String, Vec<OptionPosition>), OptionError> {
        let parsed = url::parse_optionstrat_url(value)?;
        let legs = url::parse_optionstrat_leg_fragments(
            &parsed.underlying_display_symbol,
            &parsed.leg_fragments,
        )?;
        let contracts = legs
            .iter()
            .map(|leg| leg.contract.occ_symbol.clone())
            .collect::<Vec<_>>();
        let snapshots = self.options(&contracts).await.map_err(|error| {
            OptionError::new("provider_snapshot_fetch_failed", error.to_string())
        })?;

        let mut positions = Vec::with_capacity(legs.len());
        for leg in legs {
            let snapshot = snapshots
                .get(&leg.contract.occ_symbol)
                .ok_or_else(|| {
                    OptionError::new(
                        "missing_provider_snapshot",
                        format!("missing snapshot for {}", leg.contract.occ_symbol),
                    )
                })?
                .clone();

            positions.push(OptionPosition {
                contract: leg.contract.occ_symbol.clone(),
                snapshot,
                qty: match leg.order_side {
                    OrderSide::Buy => leg.ratio_quantity as i32,
                    OrderSide::Sell => -(leg.ratio_quantity as i32),
                },
                avg_cost: alpaca_core::decimal::from_f64(
                    leg.premium_per_contract.unwrap_or(0.0),
                    2,
                ),
                leg_type: match leg.order_side {
                    OrderSide::Buy => format!("long{}", leg.contract.option_right.as_str()),
                    OrderSide::Sell => format!("short{}", leg.contract.option_right.as_str()),
                },
            });
        }

        Ok((parsed.underlying_display_symbol, positions))
    }

    pub async fn cash_dividends_total(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<f64> {
        let response = self
            .sdk()
            .corporate_actions()
            .list_all(ListRequest {
                symbols: Some(vec![symbol.to_owned()]),
                cusips: None,
                types: Some(vec![CorporateActionType::CashDividend]),
                start: Some(start_date.to_string()),
                end: Some(end_date.to_string()),
                ids: None,
                limit: Some(1000),
                sort: None,
                page_token: None,
            })
            .await
            .context("failed to load cash dividends via alpaca-data")?;

        Ok(response
            .corporate_actions
            .cash_dividends
            .iter()
            .filter_map(|dividend| dividend.rate.to_f64())
            .sum())
    }

    pub async fn chain(
        &self,
        underlying_symbol: &str,
        request: OptionChainRequest,
    ) -> Result<OptionChain> {
        self.fetch_cached_chain(underlying_symbol, request).await
    }

    pub async fn day_bars(&self, symbols: &[String]) -> BarsMap {
        self.ensure_bars(symbols, BarsWindow::Day).await
    }

    pub async fn day_bar(&self, symbol: &str) -> Option<Vec<BarPoint>> {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return None;
        }

        let requested = vec![symbol.to_string()];
        self.day_bars(&requested).await.remove(symbol)
    }

    pub async fn stats(&self) -> CacheStats {
        let raw = self.raw.stats().await;
        let options = self.options.read().await;
        let chains = self.chains.read().await;
        Self::compose_stats(raw, &options, &chains)
    }

    pub async fn watch_options(&self, contracts: &[String]) {
        let contracts = Self::normalize_option_symbols(contracts);
        if contracts.is_empty() {
            return;
        }

        self.raw.watch_options(&contracts).await;

        let mut cache = self.options.write().await;
        cache.subscribed.extend(contracts);
    }

    pub async fn watch_chains(&self, chains: &[(String, OptionChainRequest)]) {
        if chains.is_empty() {
            return;
        }

        let mut cache = self.chains.write().await;
        for (symbol, request) in chains {
            let symbol = stocks::display_stock_symbol(symbol);
            Self::merge_chain_request(&mut cache.subscribed, &symbol, request);
        }
    }

    pub async fn refresh_contracts(&self) -> Result<usize> {
        let contracts = {
            let cache = self.options.read().await;
            cache.subscribed.iter().cloned().collect::<Vec<_>>()
        };

        if contracts.is_empty() {
            return Ok(0);
        }

        self.raw.watch_options(&contracts).await;

        if let Err(error) = self.raw.refresh_options().await {
            tracing::warn!(
                "failed to refresh raw option snapshots, keeping stale cache: {}",
                error
            );
            return Ok(0);
        }

        match self.rebuild_options().await {
            Ok(count) => Ok(count),
            Err(error) => {
                tracing::warn!(
                    "failed to rebuild enriched option cache, keeping stale cache: {}",
                    error
                );
                Ok(0)
            }
        }
    }

    pub async fn refresh_chains(&self) -> Result<usize> {
        let _ = self.refresh_contracts().await?;

        let requests = {
            let cache = self.chains.read().await;
            cache
                .subscribed
                .iter()
                .map(|(symbol, request)| (symbol.clone(), request.clone()))
                .collect::<Vec<_>>()
        };

        if requests.is_empty() {
            return Ok(0);
        }

        let tasks = requests.into_iter().map(|(symbol, request)| async move {
            let underlying_price = self
                .raw
                .stock(&symbol)
                .await
                .and_then(|snapshot| snapshot.price().and_then(|value| value.to_f64()));
            let request = request.with_underlying_price(underlying_price);

            match self.fetch_and_build_chain(&symbol, &request).await {
                Ok(chain) => Some((symbol, request, chain)),
                Err(error) => {
                    tracing::warn!(
                        "failed to refresh {} option chain, keeping stale cache: {}",
                        symbol,
                        error
                    );
                    None
                }
            }
        });

        let results = futures::future::join_all(tasks).await;
        let mut count = 0;
        let mut cache = self.chains.write().await;
        for result in results.into_iter().flatten() {
            let (symbol, request, chain) = result;
            cache.cached_params.insert(symbol.clone(), request);
            cache.values.insert(symbol, chain);
            count += 1;
        }
        if count > 0 {
            cache.updated_at = Some(Self::now_timestamp());
        }

        Ok(count)
    }

    pub async fn refresh_day_bars(&self) -> Result<usize> {
        self.refresh_bars(BarsWindow::Day).await
    }

    pub async fn refresh_week_bars(&self) -> Result<usize> {
        self.refresh_bars(BarsWindow::Week).await
    }

    pub async fn refresh_month_bars(&self) -> Result<usize> {
        self.refresh_bars(BarsWindow::Month).await
    }

    pub async fn clear_cache(&self) {
        self.raw.clear_options().await;

        {
            let mut cache = self.options.write().await;
            cache.subscribed.clear();
            cache.values.clear();
            cache.empty.clear();
            cache.updated_at = None;
        }

        {
            let mut cache = self.chains.write().await;
            cache.subscribed.clear();
            cache.values.clear();
            cache.cached_params.clear();
            cache.updated_at = None;
        }

        tracing::info!(
            "[MarketCache] cleared option facade caches while keeping raw stock and bar caches"
        );
    }

    async fn ensure_options<S: AsRef<str>>(
        &self,
        contracts: &[S],
    ) -> Result<HashMap<String, OptionSnapshot>> {
        let requested = Self::normalize_option_symbols(contracts);
        if requested.is_empty() {
            return Ok(HashMap::new());
        }

        let (mut hits, missing) = {
            let cache = self.options.read().await;
            Self::collect_cached_hits(&requested, &cache.values, &cache.empty)
        };

        if !missing.is_empty() {
            let fetched = self.enrich_options(&missing).await?;
            let mut cache = self.options.write().await;
            cache.subscribed.extend(requested.iter().cloned());
            for contract in &missing {
                if fetched.contains_key(contract) {
                    cache.empty.remove(contract);
                } else {
                    cache.empty.insert(contract.clone());
                }
            }
            for (contract, snapshot) in &fetched {
                cache.values.insert(contract.clone(), snapshot.clone());
            }
            if !fetched.is_empty() {
                cache.updated_at = Some(Self::now_timestamp());
            }
            hits.extend(fetched);
        }

        Ok(requested
            .into_iter()
            .filter_map(|contract| hits.remove_entry(&contract))
            .collect())
    }

    async fn rebuild_options(&self) -> Result<usize> {
        let contracts = {
            let cache = self.options.read().await;
            cache.subscribed.iter().cloned().collect::<Vec<_>>()
        };

        if contracts.is_empty() {
            return Ok(0);
        }

        let snapshots = self.enrich_options(&contracts).await?;
        let count = snapshots.len();
        let empty = contracts
            .iter()
            .filter(|contract| !snapshots.contains_key(*contract))
            .cloned()
            .collect::<HashSet<_>>();

        let mut cache = self.options.write().await;
        cache.values = snapshots;
        cache.empty = empty;
        cache.updated_at = Some(Self::now_timestamp());
        Ok(count)
    }

    async fn enrich_options<S: AsRef<str>>(
        &self,
        contracts: &[S],
    ) -> Result<HashMap<String, OptionSnapshot>> {
        let contracts = Self::normalize_option_symbols(contracts);
        if contracts.is_empty() {
            return Ok(HashMap::new());
        }

        let snapshots = self
            .raw
            .options(&contracts)
            .await
            .context("failed to load option snapshots via alpaca-data")?;
        if snapshots.is_empty() {
            return Ok(HashMap::new());
        }

        let stock_prices = self
            .stock_prices_for(&snapshots)
            .await
            .context("failed to load underlying stock prices via alpaca-data")?;
        let (risk_free_rate, dividend_yield) = self.option_pricing_inputs();
        let stock_prices = (!stock_prices.is_empty()).then_some(&stock_prices);

        Ok(map_live_snapshots(
            &snapshots,
            self.sdk(),
            stock_prices,
            Some(risk_free_rate),
            Some(dividend_yield),
        )
        .await
        .context("failed to map option snapshots into alpaca-option models")?
        .into_iter()
        .map(|snapshot| {
            (
                snapshot.contract.occ_symbol.clone(),
                OptionSnapshot::from(snapshot),
            )
        })
        .collect())
    }

    async fn stock_prices_for(
        &self,
        snapshots: &HashMap<String, alpaca_data::options::Snapshot>,
    ) -> Result<HashMap<String, f64>> {
        let symbols = required_underlying_display_symbols(snapshots);
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        Ok(self
            .raw
            .stocks(&symbols)
            .await
            .context("failed to load stock snapshots via alpaca-data")?
            .into_iter()
            .filter_map(|(symbol, snapshot)| {
                snapshot
                    .price()
                    .and_then(|price| price.to_f64())
                    .map(|price| (symbol, price))
            })
            .collect())
    }

    async fn fetch_and_build_chain(
        &self,
        underlying_symbol: &str,
        request: &OptionChainRequest,
    ) -> Result<OptionChain> {
        let (risk_free_rate, dividend_yield) = self.option_pricing_inputs();
        let chain = crate::fetch_chain(
            self.sdk(),
            underlying_symbol,
            request,
            Some(risk_free_rate),
            Some(dividend_yield),
        )
        .await
        .context("failed to fetch and build option chain via alpaca-facade")?;

        Ok(OptionChain {
            underlying_symbol: chain.underlying_symbol,
            as_of: chain.as_of,
            snapshots: chain
                .snapshots
                .into_iter()
                .map(OptionSnapshot::from)
                .collect(),
        })
    }

    async fn fetch_cached_chain(
        &self,
        underlying_symbol: &str,
        request: OptionChainRequest,
    ) -> Result<OptionChain> {
        let symbol = stocks::display_stock_symbol(underlying_symbol);

        let cached = {
            let cache = self.chains.read().await;
            match cache.values.get(&symbol) {
                Some(chain) => {
                    let covers = match cache.cached_params.get(&symbol) {
                        Some(cached_request) => cached_request.covers(&request),
                        None => false,
                    };
                    if covers { Some(chain.clone()) } else { None }
                }
                None => None,
            }
        };

        if let Some(chain) = cached {
            self.merge_chain_subscription(&symbol, &request).await;
            return Ok(chain);
        }

        let underlying_price = self
            .raw
            .stock(&symbol)
            .await
            .and_then(|snapshot| snapshot.price().and_then(|value| value.to_f64()));
        let request = request.with_underlying_price(underlying_price);
        let chain = self.fetch_and_build_chain(&symbol, &request).await?;

        let mut cache = self.chains.write().await;
        Self::merge_chain_request(&mut cache.subscribed, &symbol, &request);
        cache.cached_params.insert(symbol.clone(), request);
        cache.values.insert(symbol, chain.clone());
        cache.updated_at = Some(Self::now_timestamp());

        Ok(chain)
    }

    async fn merge_chain_subscription(&self, symbol: &str, request: &OptionChainRequest) {
        let mut cache = self.chains.write().await;
        Self::merge_chain_request(&mut cache.subscribed, symbol, request);
    }

    fn merge_chain_request(
        map: &mut HashMap<String, OptionChainRequest>,
        symbol: &str,
        request: &OptionChainRequest,
    ) {
        map.entry(symbol.to_string())
            .and_modify(|current| current.merge(request))
            .or_insert(request.clone());
    }

    fn compose_stats(raw: RawCacheStats, options: &OptionCache, chains: &ChainCache) -> CacheStats {
        CacheStats {
            subscribed_symbols: raw.subscribed_symbols,
            subscribed_contracts: options.subscribed.len(),
            subscribed_chains: chains.subscribed.len(),
            subscribed_bar_requests: raw.subscribed_bar_requests,
            cached_stocks: raw.cached_stocks,
            cached_options: options.values.len(),
            cached_chains: chains.values.len(),
            cached_bar_symbols: raw.cached_bar_symbols,
            stocks_updated_at: raw.stocks_updated_at,
            options_updated_at: raw.options_updated_at,
            contracts_updated_at: Self::format_datetime(options.updated_at),
            chains_updated_at: Self::format_datetime(chains.updated_at),
            bars_updated_at: raw.bars_updated_at,
        }
    }

    async fn ensure_bars(&self, symbols: &[String], window: BarsWindow) -> BarsMap {
        let requested = Self::normalize_stock_symbols(symbols);
        if requested.is_empty() {
            return HashMap::new();
        }

        let resolved = requested
            .iter()
            .map(|(_, symbol)| symbol.clone())
            .collect::<Vec<_>>();
        self.raw
            .watch_bars(Self::bars_request(window, &resolved))
            .await;

        let bars = match self.raw.bars(window.key()).await {
            Ok(bars) => bars,
            Err(error) => {
                tracing::warn!("[{}] fetch failed: {}", window.refresh_label(), error);
                return HashMap::new();
            }
        };

        requested
            .into_iter()
            .filter_map(|(original, resolved)| {
                bars.get(&resolved).cloned().map(|bars| (original, bars))
            })
            .collect()
    }

    async fn refresh_bars(&self, window: BarsWindow) -> Result<usize> {
        match self.raw.refresh_bars(window.key()).await {
            Ok(count) => Ok(count),
            Err(error) => {
                tracing::warn!(
                    "failed to refresh {}, keeping stale cache: {}",
                    window.refresh_label(),
                    error
                );
                Ok(0)
            }
        }
    }
}
