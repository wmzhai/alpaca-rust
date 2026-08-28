use crate::{
    apply_optionstrat_premium_model, map_snapshots_with_pricing_references,
    pricing_references_for_snapshots, underlying_display_symbols,
};
use ::chrono::NaiveDateTime;
use alpaca_data::Client;
use alpaca_data::cache::{CacheStats as RawCacheStats, CachedClient, StockBarsRequest};
use alpaca_data::corporate_actions::{CorporateActionType, ListRequest, Region};
use alpaca_data::options::{ChainRequest, Snapshot as ProviderOptionSnapshot};
use alpaca_data::stocks::{
    self, BarPoint, BarsRequest, DataFeed, SnapshotsRequest as StockSnapshotsRequest, Sort,
    TimeFrame, preferred_feed as preferred_stock_feed,
};
use alpaca_option::contract;
use alpaca_option::url;
use alpaca_option::{OptionChain, OptionError, OptionPosition, OptionSnapshot, OrderSide};
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::sync::{Mutex, RwLock};

use alpaca_time::{calendar, chrono, clock, range, session};

pub type BarsMap = HashMap<String, Vec<BarPoint>>;

/// Cache metadata for the facade-level enriched option cache.
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub subscribed_symbols: usize,
    pub subscribed_contracts: usize,
    pub subscribed_bar_requests: usize,
    pub cached_stocks: usize,
    pub cached_options: usize,
    pub unavailable_stocks: usize,
    pub unavailable_raw_options: usize,
    pub unavailable_options: usize,
    pub cached_bar_symbols: usize,
    pub stocks_updated_at: Option<String>,
    pub options_updated_at: Option<String>,
    pub contracts_updated_at: Option<String>,
    pub bars_updated_at: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AlpacaDataConfig {
    pub dividend_yield: f64,
}

impl Default for AlpacaDataConfig {
    fn default() -> Self {
        Self {
            dividend_yield: 0.0,
        }
    }
}

pub async fn live_option_chain(
    client: &Client,
    request: ChainRequest,
    underlying_price: Option<Decimal>,
    dividend_yield: Option<f64>,
) -> Result<OptionChain> {
    let underlying_symbol = request.underlying_symbol.clone();
    let response = client
        .options()
        .chain_all(request)
        .await
        .context("failed to load live option chain via alpaca-data")?;

    let mut known_latest = HashMap::new();
    if let Some(price) = underlying_price.filter(|price| *price > Decimal::ZERO) {
        known_latest.insert(underlying_symbol.clone(), price);
    }

    let snapshots = map_live_snapshots_from_client(
        client,
        &response.snapshots,
        (!known_latest.is_empty()).then_some(&known_latest),
        dividend_yield,
    )
    .await?;
    let as_of = snapshots
        .iter()
        .map(|snapshot| snapshot.as_of.as_str())
        .filter(|timestamp| !timestamp.is_empty())
        .max()
        .unwrap_or_default()
        .to_string();

    Ok(OptionChain {
        underlying_symbol: underlying_symbol.to_uppercase(),
        as_of,
        snapshots,
    })
}

fn copy_positive_prices(
    prices: Option<&HashMap<String, Decimal>>,
) -> HashMap<String, Decimal> {
    prices
        .into_iter()
        .flat_map(|prices| prices.iter())
        .filter(|(_, price)| **price > Decimal::ZERO)
        .map(|(symbol, price)| (symbol.clone(), *price))
        .collect()
}

fn merge_positive_prices(
    target: &mut HashMap<String, Decimal>,
    fetched: HashMap<String, Decimal>,
) {
    for (symbol, price) in fetched {
        if price > Decimal::ZERO {
            target.entry(symbol).or_insert(price);
        }
    }
}

fn missing_symbols(symbols: &[String], prices: &HashMap<String, Decimal>) -> Vec<String> {
    symbols
        .iter()
        .filter(|symbol| !prices.contains_key(*symbol))
        .cloned()
        .collect()
}

async fn snapshot_stock_prices(
    client: &Client,
    symbols: &[String],
) -> Result<HashMap<String, Decimal>> {
    let requested = AlpacaData::normalize_stock_symbols(symbols);
    if requested.is_empty() {
        return Ok(HashMap::new());
    }
    let resolved = AlpacaData::unique_resolved_symbols(&requested);
    let snapshots = client
        .stocks()
        .snapshots(StockSnapshotsRequest {
            symbols: resolved,
            feed: Some(preferred_stock_feed(session::is_overnight_window(
                &clock::now(),
            ))),
            currency: None,
        })
        .await
        .context("failed to load stock snapshots via alpaca-data")?;
    Ok(requested
        .into_iter()
        .filter_map(|(original, resolved)| {
            snapshots
                .get(&resolved)
                .or_else(|| snapshots.get(&original))
                .and_then(alpaca_data::stocks::Snapshot::price)
                .filter(|price| *price > Decimal::ZERO)
                .map(|price| (original, price))
        })
        .collect())
}

async fn prices_for_iv_calculation(
    client: &Client,
    symbols: &[String],
) -> Result<HashMap<String, Decimal>> {
    if session::is_regular_session_at(&clock::now()) {
        return snapshot_stock_prices(client, symbols).await;
    }
    let requested = AlpacaData::normalize_stock_symbols(symbols);
    if requested.is_empty() {
        return Ok(HashMap::new());
    }
    close_prices_from_client(client, &requested).await
}

pub async fn map_live_snapshots_from_client(
    client: &Client,
    snapshots: &HashMap<String, ProviderOptionSnapshot>,
    known_prices: Option<&HashMap<String, Decimal>>,
    dividend_yield: Option<f64>,
) -> Result<Vec<OptionSnapshot>> {
    let now = clock::now();
    let symbols = underlying_display_symbols(snapshots);
    let iv_prices = if session::is_regular_session_at(&now) {
        let mut prices = copy_positive_prices(known_prices);
        let missing = missing_symbols(&symbols, &prices);
        if !missing.is_empty() {
            merge_positive_prices(
                &mut prices,
                snapshot_stock_prices(client, &missing).await?,
            );
        }
        prices
    } else if symbols.is_empty() {
        HashMap::new()
    } else {
        prices_for_iv_calculation(client, &symbols).await?
    };
    let price_map = (!iv_prices.is_empty()).then_some(&iv_prices);

    let pricing_references = pricing_references_for_snapshots(
        snapshots,
        price_map,
        price_map,
        &now,
    )?;
    map_snapshots_with_pricing_references(
        snapshots,
        (!pricing_references.is_empty()).then_some(&pricing_references),
        dividend_yield,
    )
    .context("failed to map option snapshots into alpaca-option models")
}

async fn close_prices_from_client(
    client: &Client,
    requested: &[(String, String)],
) -> Result<HashMap<String, Decimal>> {
    let symbols = AlpacaData::unique_resolved_symbols(requested);
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    let completed_date = calendar::last_completed_trading_date(Some(&clock::now()))
        .context("failed to resolve last completed trading date")?;
    let end = range::add_days(&completed_date, 1)
        .context("failed to build completed daily bar end date")?;
    let bars = client
        .stocks()
        .bars_all(BarsRequest {
            symbols,
            timeframe: TimeFrame::day_1(),
            start: Some(completed_date.clone()),
            end: Some(end),
            limit: Some(1000),
            adjustment: None,
            // Regular-session closes only; overnight BOATS daily bars can land on the next UTC date.
            feed: Some(DataFeed::Sip),
            sort: Some(Sort::Asc),
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .context("failed to load completed daily stock bars via alpaca-data")?
        .bars;

    Ok(requested
        .iter()
        .filter_map(|(original, resolved)| {
            let close = bars.get(resolved).and_then(|values| {
                values
                    .iter()
                    .filter_map(|bar| {
                        let close = bar.c.filter(|price| *price > Decimal::ZERO)?;
                        let timestamp = bar.t.as_deref().unwrap_or_default();
                        timestamp
                            .starts_with(&completed_date)
                            .then_some((timestamp, close))
                    })
                    .max_by(|(left_timestamp, _), (right_timestamp, _)| {
                        left_timestamp.cmp(right_timestamp)
                    })
                    .map(|(_, close)| close)
            })?;
            Some((original.clone(), close))
        })
        .collect())
}

#[derive(Default)]
struct OptionCache {
    subscribed: HashSet<String>,
    values: HashMap<String, OptionSnapshot>,
    empty: HashSet<String>,
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
    option_operations: Mutex<()>,
    options: RwLock<OptionCache>,
}

impl AlpacaData {
    #[must_use]
    pub fn with_raw(raw: CachedClient, config: AlpacaDataConfig) -> Self {
        Self {
            raw,
            config,
            option_operations: Mutex::new(()),
            options: RwLock::new(OptionCache::default()),
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

    fn option_pricing_inputs(&self) -> f64 {
        self.config.dividend_yield
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
        let _operation = self.option_operations.lock().await;
        let parsed = url::parse_optionstrat_url(value)?;
        let legs = url::parse_optionstrat_leg_fragments(
            &parsed.underlying_display_symbol,
            &parsed.leg_fragments,
        )?;
        let contracts = legs
            .iter()
            .map(|leg| leg.contract.occ_symbol.clone())
            .collect::<Vec<_>>();
        let raw_snapshots = self.raw.options(&contracts).await.map_err(|error| {
            OptionError::new("provider_snapshot_fetch_failed", error.to_string())
        })?;
        let pricing_references = self
            .iv_calculation_pricing_references(&raw_snapshots)
            .await?;
        let dividend_yield = self.option_pricing_inputs();
        let snapshots = map_snapshots_with_pricing_references(
            &raw_snapshots,
            Some(&pricing_references),
            Some(dividend_yield),
        )?
        .into_iter()
        .map(|snapshot| (snapshot.contract.occ_symbol.clone(), snapshot))
        .collect::<HashMap<_, _>>();

        let mut positions = Vec::with_capacity(legs.len());
        for leg in legs {
            let mut snapshot = snapshots
                .get(&leg.contract.occ_symbol)
                .ok_or_else(|| {
                    OptionError::new(
                        "missing_provider_snapshot",
                        format!("missing snapshot for {}", leg.contract.occ_symbol),
                    )
                })?
                .clone();
            apply_optionstrat_premium_model(
                &mut snapshot,
                leg.premium_per_contract,
                pricing_references.get(&leg.contract.occ_symbol),
                Some(dividend_yield),
            )?;
            let avg_cost = leg.premium_per_contract.unwrap_or_else(|| snapshot.price());

            positions.push(OptionPosition {
                contract: leg.contract.occ_symbol.clone(),
                snapshot,
                qty: match leg.order_side {
                    OrderSide::Buy => leg.ratio_quantity as i32,
                    OrderSide::Sell => -(leg.ratio_quantity as i32),
                },
                avg_cost: alpaca_core::decimal::from_f64(avg_cost, 2),
                leg_type: match leg.order_side {
                    OrderSide::Buy => format!("long{}", leg.contract.option_right.as_str()),
                    OrderSide::Sell => format!("short{}", leg.contract.option_right.as_str()),
                },
                option_right: None,
                strike: None,
                valuation_years: None,
            });
        }

        Ok((parsed.underlying_display_symbol, positions))
    }

    async fn iv_calculation_pricing_references(
        &self,
        snapshots: &HashMap<String, ProviderOptionSnapshot>,
    ) -> Result<HashMap<String, crate::OptionPricingReference>, OptionError> {
        let now = clock::now();
        let symbols = underlying_display_symbols(snapshots);
        if symbols.is_empty() {
            return pricing_references_for_snapshots(snapshots, None, None, &now);
        }
        let iv_prices = self
            .get_prices_for_iv_calculation(&symbols)
            .await
            .map_err(|error| {
                OptionError::new("provider_stock_price_fetch_failed", error.to_string())
            })?;
        let price_map = (!iv_prices.is_empty()).then_some(&iv_prices);
        pricing_references_for_snapshots(snapshots, price_map, price_map, &now)
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
                region: Some(Region::Us),
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

    pub async fn get_prices_for_iv_calculation<S: AsRef<str>>(
        &self,
        symbols: &[S],
    ) -> Result<HashMap<String, Decimal>> {
        if session::is_regular_session_at(&clock::now()) {
            let snapshots = self
                .raw
                .stocks(symbols)
                .await
                .context("failed to load stock snapshots via alpaca-data")?;
            Ok(snapshots
                .into_iter()
                .filter_map(|(symbol, snapshot)| {
                    snapshot
                        .price()
                        .filter(|price| *price > Decimal::ZERO)
                        .map(|price| (symbol, price))
                })
                .collect())
        } else {
            let requested = Self::normalize_stock_symbols(symbols);
            self.close_prices(&requested).await
        }
    }

    pub async fn stats(&self) -> CacheStats {
        let raw = self.raw.stats().await;
        let options = self.options.read().await;
        Self::compose_stats(raw, &options)
    }

    pub async fn watch_options(&self, contracts: &[String]) {
        let contracts = Self::normalize_option_symbols(contracts);
        if contracts.is_empty() {
            return;
        }

        let _operation = self.option_operations.lock().await;
        self.raw.watch_options(&contracts).await;

        let mut cache = self.options.write().await;
        cache.subscribed.extend(contracts);
    }

    pub async fn refresh_contracts(&self) -> Result<usize> {
        let _operation = self.option_operations.lock().await;
        let contracts = {
            let cache = self.options.read().await;
            cache.subscribed.iter().cloned().collect::<Vec<_>>()
        };

        if contracts.is_empty() {
            return Ok(0);
        }

        self.raw.watch_options(&contracts).await;

        self.raw
            .refresh_options()
            .await
            .context("failed to refresh raw option snapshots")?;
        self.rebuild_options()
            .await
            .context("failed to rebuild enriched option cache")
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
        let _operation = self.option_operations.lock().await;
        self.raw.clear_options().await;

        {
            let mut cache = self.options.write().await;
            cache.subscribed.clear();
            cache.values.clear();
            cache.empty.clear();
            cache.updated_at = None;
        }

        tracing::info!(
            "[MarketCache] cleared option facade cache while keeping raw stock and bar caches"
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
            let _operation = self.option_operations.lock().await;
            let current = {
                let cache = self.options.read().await;
                Self::collect_cached_hits(&requested, &cache.values, &cache.empty)
            };
            hits = current.0;
            let missing = current.1;
            if missing.is_empty() {
                return Ok(hits);
            }

            let fetched = self.enrich_options(&missing).await?;
            let mut cache = self.options.write().await;
            cache.subscribed.extend(requested.iter().cloned());
            for contract in &missing {
                if let Some(snapshot) = fetched.get(contract) {
                    cache.values.insert(contract.clone(), snapshot.clone());
                    cache.empty.remove(contract);
                    hits.insert(contract.clone(), snapshot.clone());
                } else {
                    cache.values.remove(contract);
                    cache.empty.insert(contract.clone());
                }
            }
            cache.updated_at = Some(Self::now_timestamp());
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

        let dividend_yield = self.option_pricing_inputs();

        Ok(self
            .map_live_snapshots(&snapshots, None, Some(dividend_yield))
            .await?
            .into_iter()
            .map(|snapshot| {
                (
                    snapshot.contract.occ_symbol.clone(),
                    OptionSnapshot::from(snapshot),
                )
            })
            .collect())
    }

    pub async fn map_live_snapshots(
        &self,
        snapshots: &HashMap<String, alpaca_data::options::Snapshot>,
        known_prices: Option<&HashMap<String, Decimal>>,
        dividend_yield: Option<f64>,
    ) -> Result<Vec<OptionSnapshot>> {
        let now = clock::now();
        let symbols = underlying_display_symbols(snapshots);
        let iv_prices = if session::is_regular_session_at(&now) {
            let mut prices = copy_positive_prices(known_prices);
            let missing = missing_symbols(&symbols, &prices);
            if !missing.is_empty() {
                merge_positive_prices(
                    &mut prices,
                    self.get_prices_for_iv_calculation(&missing)
                        .await
                        .context("failed to load underlying stock prices for options")?,
                );
            }
            prices
        } else if symbols.is_empty() {
            HashMap::new()
        } else {
            self.get_prices_for_iv_calculation(&symbols)
                .await
                .context("failed to load IV calculation stock prices for options")?
        };
        let price_map = (!iv_prices.is_empty()).then_some(&iv_prices);

        let pricing_references = pricing_references_for_snapshots(
            snapshots,
            price_map,
            price_map,
            &now,
        )?;
        map_snapshots_with_pricing_references(
            snapshots,
            (!pricing_references.is_empty()).then_some(&pricing_references),
            dividend_yield,
        )
        .context("failed to map option snapshots into alpaca-option models")
    }

    async fn close_prices(
        &self,
        requested: &[(String, String)],
    ) -> Result<HashMap<String, Decimal>> {
        close_prices_from_client(self.sdk(), requested).await
    }

    fn compose_stats(raw: RawCacheStats, options: &OptionCache) -> CacheStats {
        CacheStats {
            subscribed_symbols: raw.subscribed_symbols,
            subscribed_contracts: options.subscribed.len(),
            subscribed_bar_requests: raw.subscribed_bar_requests,
            cached_stocks: raw.cached_stocks,
            cached_options: options.values.len(),
            unavailable_stocks: raw.unavailable_stocks,
            unavailable_raw_options: raw.unavailable_options,
            unavailable_options: options.empty.len(),
            cached_bar_symbols: raw.cached_bar_symbols,
            stocks_updated_at: raw.stocks_updated_at,
            options_updated_at: raw.options_updated_at,
            contracts_updated_at: Self::format_datetime(options.updated_at),
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
        let result = self.raw.refresh_bars(window.key()).await;
        if result.is_err() {
            tracing::warn!(
                window = window.refresh_label(),
                "market cache bar refresh failed"
            );
        }
        result.with_context(|| format!("failed to refresh {}", window.refresh_label()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_price_symbols_preserve_requested_keys_and_deduplicate_provider_symbols() {
        let requested = AlpacaData::normalize_stock_symbols(&["BRK.B", "BRKB", " SPY ", "SPY"]);

        assert_eq!(
            requested,
            vec![
                ("BRK.B".to_owned(), "BRK.B".to_owned()),
                ("BRKB".to_owned(), "BRK.B".to_owned()),
                ("SPY".to_owned(), "SPY".to_owned()),
            ]
        );
        assert_eq!(
            AlpacaData::unique_resolved_symbols(&requested),
            vec!["BRK.B".to_owned(), "SPY".to_owned()]
        );
    }

}
