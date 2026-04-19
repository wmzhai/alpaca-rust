#![forbid(unsafe_code)]

//! alpaca-facade
//!
//! Thin bridge helpers that map Alpaca option payloads into `alpaca-option`
//! core models.

use std::collections::HashMap;

use alpaca_core::decimal;
use alpaca_data::Client;
use alpaca_data::options::{
    ChainRequest, ContractType, OptionsFeed, Snapshot, SnapshotsRequest, ordered_snapshots,
    preferred_feed,
};
use alpaca_data::stocks::{DataFeed, display_stock_symbol};
use alpaca_option::contract;
use alpaca_option::pricing;
use alpaca_option::url;
use alpaca_option::{
    Greeks, OptionChain, OptionError, OptionPosition, OptionQuote, OptionResult, OptionRight,
    OptionSnapshot, StrategyLegInput,
};
use alpaca_time::clock;
use alpaca_time::expiration;
use alpaca_time::session;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};

const GREEKS_EPSILON: f64 = 1e-10;
const DEFAULT_RISK_FREE_RATE: f64 = 0.0362;
const DEFAULT_DIVIDEND_YIELD: f64 = 0.0;
const MAX_INFERRED_IV: f64 = 2.0;
const MIN_TIME_YEARS: f64 = 0.0001;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedOptionStratPositions {
    pub underlying_display_symbol: String,
    pub legs: Vec<StrategyLegInput>,
    pub positions: Vec<OptionPosition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionChainRequest {
    option_type: Option<OptionRight>,
    strike_price_gte: Option<Decimal>,
    strike_price_lte: Option<Decimal>,
    expiration_date_gte: Option<String>,
    expiration_date_lte: Option<String>,
    underlying_price: Option<f64>,
}

impl OptionChainRequest {
    #[must_use]
    pub fn new() -> Self {
        Self {
            option_type: None,
            strike_price_gte: None,
            strike_price_lte: None,
            expiration_date_gte: None,
            expiration_date_lte: None,
            underlying_price: None,
        }
    }

    #[must_use]
    pub fn from_dte_range(
        min_dte: i32,
        max_dte: i32,
        strike_price_gte: Option<f64>,
        strike_price_lte: Option<f64>,
    ) -> Self {
        let today = clock::today();

        Self::from_expiration_range(
            Some(&alpaca_time::range::add_days(&today, min_dte).unwrap_or_else(|_| today.clone())),
            Some(&alpaca_time::range::add_days(&today, max_dte).unwrap_or(today)),
        )
        .with_strike_range(strike_price_gte, strike_price_lte)
    }

    #[must_use]
    pub fn from_expiration_range(
        expiration_date_gte: Option<&str>,
        expiration_date_lte: Option<&str>,
    ) -> Self {
        Self::new().with_expiration_range(expiration_date_gte, expiration_date_lte)
    }

    #[must_use]
    pub fn with_strike_range(
        mut self,
        strike_price_gte: Option<f64>,
        strike_price_lte: Option<f64>,
    ) -> Self {
        self.strike_price_gte = strike_price_gte.map(|value| decimal::from_f64(value, 2));
        self.strike_price_lte = strike_price_lte.map(|value| decimal::from_f64(value, 2));
        self
    }

    #[must_use]
    pub fn with_expiration_range(
        mut self,
        expiration_date_gte: Option<&str>,
        expiration_date_lte: Option<&str>,
    ) -> Self {
        self.expiration_date_gte = expiration_date_gte.map(str::to_string);
        self.expiration_date_lte = expiration_date_lte.map(str::to_string);
        self
    }

    #[must_use]
    pub fn with_option_type(mut self, option_type: OptionRight) -> Self {
        self.option_type = Some(option_type);
        self
    }

    #[must_use]
    pub fn with_underlying_price(mut self, underlying_price: Option<f64>) -> Self {
        if self.underlying_price.is_none()
            && matches!(underlying_price, Some(value) if value.is_finite() && value > 0.0)
        {
            self.underlying_price = underlying_price;
        }
        self
    }

    #[must_use]
    pub fn has_filters(&self) -> bool {
        self.option_type.is_some()
            || self.strike_price_gte.is_some()
            || self.strike_price_lte.is_some()
            || self.expiration_date_gte.is_some()
            || self.expiration_date_lte.is_some()
    }

    #[must_use]
    pub fn option_type(&self) -> Option<&OptionRight> {
        self.option_type.as_ref()
    }

    #[must_use]
    pub fn strike_price_gte(&self) -> Option<Decimal> {
        self.strike_price_gte
    }

    #[must_use]
    pub fn strike_price_lte(&self) -> Option<Decimal> {
        self.strike_price_lte
    }

    #[must_use]
    pub fn expiration_date_gte(&self) -> Option<&str> {
        self.expiration_date_gte.as_deref()
    }

    #[must_use]
    pub fn expiration_date_lte(&self) -> Option<&str> {
        self.expiration_date_lte.as_deref()
    }

    #[must_use]
    pub fn underlying_price(&self) -> Option<f64> {
        self.underlying_price
    }

    fn provider_request(&self, underlying_symbol: &str) -> ChainRequest {
        ChainRequest {
            underlying_symbol: underlying_symbol.to_owned(),
            feed: Some(preferred_feed()),
            r#type: provider_contract_type(self.option_type.as_ref()),
            strike_price_gte: self.strike_price_gte,
            strike_price_lte: self.strike_price_lte,
            expiration_date: None,
            expiration_date_gte: self.expiration_date_gte.clone(),
            expiration_date_lte: self.expiration_date_lte.clone(),
            root_symbol: None,
            updated_since: None,
            limit: Some(1000),
            page_token: None,
        }
    }

    #[must_use]
    pub fn covers(&self, requested: &Self) -> bool {
        option_type_covers(&self.option_type, &requested.option_type)
            && lower_bound_covers(self.strike_price_gte, requested.strike_price_gte)
            && upper_bound_covers(self.strike_price_lte, requested.strike_price_lte)
            && lower_bound_covers(
                self.expiration_date_gte.as_deref(),
                requested.expiration_date_gte.as_deref(),
            )
            && upper_bound_covers(
                self.expiration_date_lte.as_deref(),
                requested.expiration_date_lte.as_deref(),
            )
    }

    pub fn merge(&mut self, other: &Self) {
        if self.option_type != other.option_type {
            self.option_type = None;
        }

        self.strike_price_gte = merged_lower_bound(self.strike_price_gte, other.strike_price_gte);
        self.strike_price_lte = merged_upper_bound(self.strike_price_lte, other.strike_price_lte);
        self.expiration_date_gte = merged_lower_bound(
            self.expiration_date_gte.clone(),
            other.expiration_date_gte.clone(),
        );
        self.expiration_date_lte = merged_upper_bound(
            self.expiration_date_lte.clone(),
            other.expiration_date_lte.clone(),
        );

        if self.underlying_price.is_none() {
            self.underlying_price = other.underlying_price;
        }
    }
}

fn option_type_covers(cached: &Option<OptionRight>, requested: &Option<OptionRight>) -> bool {
    match (cached.as_ref(), requested.as_ref()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(cached), Some(requested)) => cached == requested,
    }
}

fn lower_bound_covers<T>(cached: Option<T>, requested: Option<T>) -> bool
where
    T: PartialOrd,
{
    match (cached, requested) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(cached), Some(requested)) => cached <= requested,
    }
}

fn upper_bound_covers<T>(cached: Option<T>, requested: Option<T>) -> bool
where
    T: PartialOrd,
{
    match (cached, requested) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(cached), Some(requested)) => cached >= requested,
    }
}

fn merged_lower_bound<T>(current: Option<T>, incoming: Option<T>) -> Option<T>
where
    T: PartialOrd,
{
    match (current, incoming) {
        (None, _) | (_, None) => None,
        (Some(current), Some(incoming)) => Some(if current <= incoming {
            current
        } else {
            incoming
        }),
    }
}

fn merged_upper_bound<T>(current: Option<T>, incoming: Option<T>) -> Option<T>
where
    T: PartialOrd,
{
    match (current, incoming) {
        (None, _) | (_, None) => None,
        (Some(current), Some(incoming)) => Some(if current >= incoming {
            current
        } else {
            incoming
        }),
    }
}

fn decimal_to_f64(value: Option<rust_decimal::Decimal>) -> Option<f64> {
    value.and_then(|number| number.to_f64())
}

fn provider_contract_type(option_type: Option<&OptionRight>) -> Option<ContractType> {
    match option_type {
        Some(OptionRight::Call) => Some(ContractType::Call),
        Some(OptionRight::Put) => Some(ContractType::Put),
        _ => None,
    }
}

fn snapshot_as_of(snapshot: &Snapshot) -> OptionResult<String> {
    let Some(raw_timestamp) = snapshot.timestamp() else {
        return Ok(clock::now());
    };

    Ok(clock::parse_timestamp(raw_timestamp).unwrap_or_else(|_| clock::now()))
}

fn map_quote(snapshot: &Snapshot) -> OptionQuote {
    OptionQuote {
        bid: decimal_to_f64(snapshot.bid_price()),
        ask: decimal_to_f64(snapshot.ask_price()),
        mark: decimal_to_f64(snapshot.mark_price()),
        last: decimal_to_f64(snapshot.last_price()),
    }
}

fn map_greeks(snapshot: &Snapshot) -> Option<Greeks> {
    let greeks = snapshot.greeks.as_ref()?;
    Some(Greeks {
        delta: decimal_to_f64(greeks.delta)?,
        gamma: decimal_to_f64(greeks.gamma)?,
        vega: decimal_to_f64(greeks.vega)?,
        theta: decimal_to_f64(greeks.theta)?,
        rho: decimal_to_f64(greeks.rho)?,
    })
}

fn valid_underlying_price(underlying_price: Option<f64>) -> Option<f64> {
    underlying_price.filter(|value| value.is_finite() && *value > 0.0)
}

fn valid_iv(implied_volatility: Option<f64>) -> Option<f64> {
    implied_volatility.filter(|value| value.is_finite() && *value > 0.0)
}

fn greeks_are_invalid(greeks: Option<&Greeks>) -> bool {
    match greeks {
        Some(greeks) => {
            !greeks.delta.is_finite()
                || !greeks.gamma.is_finite()
                || !greeks.theta.is_finite()
                || !greeks.vega.is_finite()
                || !greeks.rho.is_finite()
                || (greeks.delta.abs() < GREEKS_EPSILON
                    && greeks.gamma.abs() < GREEKS_EPSILON
                    && greeks.theta.abs() < GREEKS_EPSILON
                    && greeks.vega.abs() < GREEKS_EPSILON)
        }
        None => true,
    }
}

fn snapshot_needs_repair(greeks: Option<&Greeks>, implied_volatility: Option<f64>) -> bool {
    greeks_are_invalid(greeks) || valid_iv(implied_volatility).is_none()
}

fn quote_price(quote: &OptionQuote) -> Option<f64> {
    quote
        .mark
        .or(quote.last)
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn capped_low_price_greeks(
    contract: &alpaca_option::OptionContract,
    option_price: f64,
    underlying_price: f64,
    greeks: &mut Greeks,
) {
    if option_price >= 0.05 {
        return;
    }

    let estimated_delta = option_price / underlying_price;
    let max_delta_abs = (estimated_delta * 10.0).max(0.05);
    if greeks.delta.abs() > max_delta_abs {
        greeks.delta = match contract.option_right {
            alpaca_option::OptionRight::Call => estimated_delta,
            alpaca_option::OptionRight::Put => -estimated_delta,
        };
    }

    let max_theta_abs = option_price * 5.0;
    if greeks.theta.abs() > max_theta_abs {
        greeks.theta = -max_theta_abs;
    }

    let max_gamma = greeks.delta.abs() * 10.0 / underlying_price;
    if max_gamma > 0.0 && greeks.gamma > max_gamma {
        greeks.gamma = max_gamma;
    }

    let max_vega = option_price * 2.0;
    if greeks.vega > max_vega {
        greeks.vega = max_vega;
    }
}

fn repaired_greeks_and_iv(
    contract: &alpaca_option::OptionContract,
    quote: &OptionQuote,
    provider_greeks: Option<Greeks>,
    provider_iv: Option<f64>,
    underlying_price: Option<f64>,
    risk_free_rate: Option<f64>,
    dividend_yield: Option<f64>,
) -> (Option<Greeks>, Option<f64>) {
    if !snapshot_needs_repair(provider_greeks.as_ref(), provider_iv) {
        return (provider_greeks, valid_iv(provider_iv));
    }

    let fallback_greeks = (!greeks_are_invalid(provider_greeks.as_ref()))
        .then_some(provider_greeks)
        .flatten();
    let fallback_iv = valid_iv(provider_iv);
    let Some(underlying_price) = valid_underlying_price(underlying_price) else {
        return (fallback_greeks, fallback_iv);
    };

    let years =
        expiration::years(&contract.expiration_date, Some(&clock::now()), None).max(MIN_TIME_YEARS);
    let risk_free_rate = risk_free_rate.unwrap_or(DEFAULT_RISK_FREE_RATE);
    let dividend_yield = dividend_yield.unwrap_or(DEFAULT_DIVIDEND_YIELD);

    let implied_volatility = if let Some(implied_volatility) = fallback_iv {
        Some(implied_volatility)
    } else {
        quote_price(quote).and_then(|option_price| {
            pricing::implied_volatility_from_price(
                &alpaca_option::BlackScholesImpliedVolatilityInput {
                    target_price: option_price,
                    spot: underlying_price,
                    strike: contract.strike,
                    years,
                    rate: risk_free_rate,
                    dividend_yield,
                    option_right: contract.option_right.clone(),
                    lower_bound: None,
                    upper_bound: None,
                    tolerance: None,
                    max_iterations: None,
                },
            )
            .ok()
            .map(|value| value.min(MAX_INFERRED_IV))
        })
    };

    let Some(implied_volatility) = implied_volatility else {
        return (fallback_greeks, fallback_iv);
    };

    let mut greeks = match pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: underlying_price,
        strike: contract.strike,
        years,
        rate: risk_free_rate,
        dividend_yield,
        volatility: implied_volatility,
        option_right: contract.option_right.clone(),
    }) {
        Ok(greeks) => greeks,
        Err(_) => return (fallback_greeks, Some(implied_volatility)),
    };

    if let Some(option_price) = quote_price(quote) {
        capped_low_price_greeks(contract, option_price, underlying_price, &mut greeks);
    }

    (Some(greeks), Some(implied_volatility))
}

fn lookup_underlying_price(
    occ_symbol: &str,
    underlying_prices: Option<&HashMap<String, f64>>,
) -> Option<f64> {
    let underlying_prices = underlying_prices?;
    let contract = contract::parse_occ_symbol(occ_symbol)?;
    let display_symbol = display_stock_symbol(&contract.underlying_symbol);
    valid_underlying_price(
        underlying_prices
            .get(&contract.underlying_symbol)
            .copied()
            .or_else(|| underlying_prices.get(&display_symbol).copied()),
    )
}

pub async fn fetch_chain(
    client: &Client,
    underlying_symbol: &str,
    request: &OptionChainRequest,
    risk_free_rate: Option<f64>,
    dividend_yield: Option<f64>,
) -> OptionResult<OptionChain> {
    let response = client
        .options()
        .chain_all(request.provider_request(underlying_symbol))
        .await
        .map_err(|error| {
            OptionError::new(
                "alpaca_option_chain_failed",
                format!("failed to fetch alpaca option chain: {error}"),
            )
        })?;

    let mut underlying_prices = HashMap::new();
    if let Some(underlying_price) = valid_underlying_price(request.underlying_price()) {
        underlying_prices.insert(underlying_symbol.to_owned(), underlying_price);
    }

    let snapshots = map_snapshots(
        &response.snapshots,
        (!underlying_prices.is_empty()).then_some(&underlying_prices),
        risk_free_rate,
        dividend_yield,
    )?;

    let as_of = snapshots
        .iter()
        .map(|snapshot| snapshot.as_of.as_str())
        .filter(|timestamp| !timestamp.is_empty())
        .max()
        .map(str::to_string)
        .unwrap_or_default();

    Ok(OptionChain {
        underlying_symbol: underlying_symbol.to_uppercase(),
        as_of,
        snapshots,
    })
}

pub fn map_snapshot(
    occ_symbol: &str,
    snapshot: &Snapshot,
    underlying_price: Option<f64>,
    risk_free_rate: Option<f64>,
    dividend_yield: Option<f64>,
) -> OptionResult<OptionSnapshot> {
    let contract = contract::parse_occ_symbol(occ_symbol).ok_or_else(|| {
        OptionError::new(
            "invalid_occ_symbol",
            format!("invalid occ symbol: {occ_symbol}"),
        )
    })?;
    let quote = map_quote(snapshot);
    let provider_greeks = map_greeks(snapshot);
    let provider_iv = decimal_to_f64(snapshot.implied_volatility);
    let (greeks, implied_volatility) = repaired_greeks_and_iv(
        &contract,
        &quote,
        provider_greeks,
        provider_iv,
        underlying_price,
        risk_free_rate,
        dividend_yield,
    );

    Ok(OptionSnapshot {
        as_of: snapshot_as_of(snapshot)?,
        contract,
        quote,
        greeks,
        implied_volatility,
        underlying_price,
    })
}

pub fn map_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    underlying_prices: Option<&HashMap<String, f64>>,
    risk_free_rate: Option<f64>,
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    ordered_snapshots(snapshots)
        .into_iter()
        .map(|(occ_symbol, snapshot)| {
            map_snapshot(
                occ_symbol,
                snapshot,
                lookup_underlying_price(occ_symbol, underlying_prices),
                risk_free_rate,
                dividend_yield,
            )
        })
        .collect()
}

pub fn required_underlying_display_symbols(snapshots: &HashMap<String, Snapshot>) -> Vec<String> {
    let mut symbols = ordered_snapshots(snapshots)
        .into_iter()
        .filter_map(|(occ_symbol, snapshot)| {
            snapshot_needs_repair(
                map_greeks(snapshot).as_ref(),
                decimal_to_f64(snapshot.implied_volatility),
            )
            .then(|| {
                contract::parse_occ_symbol(occ_symbol)
                    .map(|contract| display_stock_symbol(&contract.underlying_symbol))
            })
            .flatten()
        })
        .collect::<Vec<_>>();
    symbols.sort_unstable();
    symbols.dedup();
    symbols
}

fn underlying_display_symbols(snapshots: &HashMap<String, Snapshot>) -> Vec<String> {
    let mut symbols = ordered_snapshots(snapshots)
        .into_iter()
        .filter_map(|(occ_symbol, _)| {
            contract::parse_occ_symbol(occ_symbol)
                .map(|contract| display_stock_symbol(&contract.underlying_symbol))
        })
        .collect::<Vec<_>>();
    symbols.sort_unstable();
    symbols.dedup();
    symbols
}

async fn fetch_underlying_prices(
    client: &Client,
    snapshots: &HashMap<String, Snapshot>,
    known_prices: Option<&HashMap<String, f64>>,
) -> OptionResult<HashMap<String, f64>> {
    let mut prices = known_prices.cloned().unwrap_or_default();
    let symbols = underlying_display_symbols(snapshots);
    let missing_symbols = symbols
        .into_iter()
        .filter(|symbol| valid_underlying_price(prices.get(symbol).copied()).is_none())
        .collect::<Vec<_>>();

    if missing_symbols.is_empty() {
        return Ok(prices);
    }

    let Ok(response) = client
        .stocks()
        .snapshots(alpaca_data::stocks::SnapshotsRequest {
            symbols: missing_symbols,
            feed: session::is_overnight_window(&clock::now()).then_some(DataFeed::Boats),
            currency: None,
        })
        .await
    else {
        return Ok(prices);
    };

    for (symbol, snapshot) in response {
        if let Some(price) = snapshot.price().and_then(|price| price.to_f64()) {
            prices.insert(symbol, price);
        }
    }
    Ok(prices)
}

pub async fn map_live_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    client: &Client,
    underlying_prices: Option<&HashMap<String, f64>>,
    risk_free_rate: Option<f64>,
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    let underlying_prices = fetch_underlying_prices(client, snapshots, underlying_prices).await?;
    map_snapshots(
        snapshots,
        (!underlying_prices.is_empty()).then_some(&underlying_prices),
        risk_free_rate,
        dividend_yield,
    )
}

pub async fn resolve_positions_from_optionstrat_url(
    value: &str,
    client: &Client,
) -> OptionResult<ResolvedOptionStratPositions> {
    let parsed = url::parse_optionstrat_url(value)?;
    let legs = url::parse_optionstrat_leg_fragments(
        &parsed.underlying_display_symbol,
        &parsed.leg_fragments,
    )?;
    let occ_symbols = legs
        .iter()
        .map(|leg| leg.contract.occ_symbol.clone())
        .collect::<Vec<_>>();
    let snapshots = client
        .options()
        .snapshots_all(SnapshotsRequest {
            symbols: occ_symbols,
            feed: Some(OptionsFeed::Opra),
            limit: Some(1000),
            page_token: None,
        })
        .await
        .map_err(|error| OptionError::new("provider_snapshot_fetch_failed", error.to_string()))?
        .snapshots;
    let mapped_snapshots = map_live_snapshots(&snapshots, client, None, None, None).await?;
    let snapshots_by_occ = mapped_snapshots
        .into_iter()
        .map(|snapshot| (snapshot.contract.occ_symbol.clone(), snapshot))
        .collect::<HashMap<_, _>>();

    let mut positions = Vec::with_capacity(legs.len());
    for leg in &legs {
        let snapshot = snapshots_by_occ
            .get(&leg.contract.occ_symbol)
            .ok_or_else(|| {
                OptionError::new(
                    "missing_provider_snapshot",
                    format!("missing snapshot for {}", leg.contract.occ_symbol),
                )
            })?;
        positions.push(OptionPosition {
            contract: leg.contract.occ_symbol.clone(),
            snapshot: snapshot.clone(),
            qty: match leg.order_side {
                alpaca_option::OrderSide::Buy => leg.ratio_quantity as i32,
                alpaca_option::OrderSide::Sell => -(leg.ratio_quantity as i32),
            },
            avg_cost: decimal::from_f64(leg.premium_per_contract.unwrap_or(0.0), 2),
            leg_type: match leg.order_side {
                alpaca_option::OrderSide::Buy => {
                    format!("long{}", leg.contract.option_right.as_str())
                }
                alpaca_option::OrderSide::Sell => {
                    format!("short{}", leg.contract.option_right.as_str())
                }
            },
        });
    }

    Ok(ResolvedOptionStratPositions {
        underlying_display_symbol: parsed.underlying_display_symbol,
        legs,
        positions,
    })
}

pub const SPEC_ADAPTER_API: &str = "spec/api/alpaca-adapter-api.md";
