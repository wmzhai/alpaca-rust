#![forbid(unsafe_code)]

//! alpaca-facade
//!
//! Thin bridge helpers that map Alpaca option payloads into `alpaca-option`
//! core models.

use std::collections::{HashMap, HashSet};

use alpaca_core::decimal;
use alpaca_data::Client;
use alpaca_data::options::{OptionsFeed, Snapshot, SnapshotsRequest, ordered_snapshots};
use alpaca_data::stocks::{BarsRequest, DataFeed, Sort, TimeFrame, display_stock_symbol};
use alpaca_option::contract;
use alpaca_option::pricing;
use alpaca_option::url;
use alpaca_option::{
    Greeks, OptionError, OptionPosition, OptionQuote, OptionResult, OptionSnapshot,
    StrategyLegInput,
};
use alpaca_time::calendar;
use alpaca_time::clock;
use alpaca_time::expiration;
use alpaca_time::range;
use alpaca_time::session;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

const GREEKS_EPSILON: f64 = 1e-10;
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
pub struct OptionPricingReference {
    pub evaluation_time: String,
    pub underlying_price: Option<f64>,
}

fn decimal_to_f64(value: Option<rust_decimal::Decimal>) -> Option<f64> {
    value.and_then(|number| number.to_f64())
}

fn normalize_timestamp_or_fallback(raw_timestamp: Option<&str>, fallback: &str) -> String {
    let fallback = clock::parse_timestamp(fallback).unwrap_or_else(|_| clock::now());
    raw_timestamp
        .and_then(|timestamp| clock::parse_timestamp(timestamp).ok())
        .unwrap_or(fallback)
}

fn snapshot_as_of_with_fallback(snapshot: &Snapshot, fallback: &str) -> String {
    normalize_timestamp_or_fallback(snapshot.timestamp(), fallback)
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

fn decimal_to_valid_price(value: Option<rust_decimal::Decimal>) -> Option<f64> {
    valid_underlying_price(decimal_to_f64(value))
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
    pricing_reference: Option<&OptionPricingReference>,
    dividend_yield: Option<f64>,
) -> (Option<Greeks>, Option<f64>) {
    if !snapshot_needs_repair(provider_greeks.as_ref(), provider_iv) {
        return (provider_greeks, valid_iv(provider_iv));
    }

    let fallback_greeks = (!greeks_are_invalid(provider_greeks.as_ref()))
        .then_some(provider_greeks)
        .flatten();
    let fallback_iv = valid_iv(provider_iv);
    let Some(pricing_reference) = pricing_reference else {
        return (fallback_greeks, fallback_iv);
    };
    let Some(underlying_price) = valid_underlying_price(pricing_reference.underlying_price) else {
        return (fallback_greeks, fallback_iv);
    };

    let years = expiration::years(
        &contract.expiration_date,
        Some(&pricing_reference.evaluation_time),
        None,
    )
    .max(MIN_TIME_YEARS);
    let dividend_yield = dividend_yield.unwrap_or(DEFAULT_DIVIDEND_YIELD);

    let implied_volatility = if let Some(implied_volatility) = fallback_iv {
        Some(implied_volatility)
    } else {
        quote_price(quote).and_then(|option_price| {
            pricing::implied_volatility_from_price(
                &alpaca_option::BlackScholesImpliedVolatilityInput::new(
                    option_price,
                    underlying_price,
                    contract.strike,
                    years,
                    dividend_yield,
                    contract.option_right.clone(),
                ),
            )
            .ok()
            .map(|value| value.min(MAX_INFERRED_IV))
        })
    };

    let Some(implied_volatility) = implied_volatility else {
        return (fallback_greeks, fallback_iv);
    };

    let mut greeks = match pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput::new(
        underlying_price,
        contract.strike,
        years,
        dividend_yield,
        implied_volatility,
        contract.option_right.clone(),
    )) {
        Ok(greeks) => greeks,
        Err(_) => return (fallback_greeks, Some(implied_volatility)),
    };

    if let Some(option_price) = quote_price(quote) {
        capped_low_price_greeks(contract, option_price, underlying_price, &mut greeks);
    }

    (Some(greeks), Some(implied_volatility))
}

fn close_evaluation_time(now: &str) -> OptionResult<String> {
    calendar::last_completed_trading_date(Some(now))
        .map(|date| format!("{date} 16:00:00"))
        .map_err(|error| OptionError::new("invalid_pricing_time", error.to_string()))
}

fn pricing_reference_for_snapshot(
    snapshot: &Snapshot,
    underlying_price: Option<f64>,
    now: &str,
) -> OptionResult<OptionPricingReference> {
    let evaluation_time = if session::is_regular_session_at(now) {
        snapshot_as_of_with_fallback(snapshot, now)
    } else {
        close_evaluation_time(now)?
    };

    Ok(OptionPricingReference {
        evaluation_time,
        underlying_price: valid_underlying_price(underlying_price),
    })
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

pub fn pricing_references_for_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    realtime_prices: Option<&HashMap<String, f64>>,
    close_prices: Option<&HashMap<String, f64>>,
    now: &str,
) -> OptionResult<HashMap<String, OptionPricingReference>> {
    let price_source = if session::is_regular_session_at(now) {
        realtime_prices
    } else {
        close_prices
    };
    ordered_snapshots(snapshots)
        .into_iter()
        .map(|(occ_symbol, snapshot)| {
            let reference = pricing_reference_for_snapshot(
                snapshot,
                lookup_underlying_price(occ_symbol, price_source),
                now,
            )?;
            Ok((occ_symbol.to_owned(), reference))
        })
        .collect()
}

pub fn map_snapshot_with_pricing_reference(
    occ_symbol: &str,
    snapshot: &Snapshot,
    pricing_reference: Option<&OptionPricingReference>,
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
        pricing_reference,
        dividend_yield,
    );

    Ok(OptionSnapshot {
        as_of: snapshot_as_of(snapshot)?,
        contract,
        quote,
        greeks,
        implied_volatility,
        underlying_price: pricing_reference.and_then(|reference| reference.underlying_price),
    })
}

pub fn map_snapshot(
    occ_symbol: &str,
    snapshot: &Snapshot,
    underlying_price: Option<f64>,
    dividend_yield: Option<f64>,
) -> OptionResult<OptionSnapshot> {
    let now = clock::now();
    let pricing_reference = pricing_reference_for_snapshot(snapshot, underlying_price, &now)?;
    map_snapshot_with_pricing_reference(
        occ_symbol,
        snapshot,
        Some(&pricing_reference),
        dividend_yield,
    )
}

pub fn map_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    underlying_prices: Option<&HashMap<String, f64>>,
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    let now = clock::now();
    let pricing_references =
        pricing_references_for_snapshots(snapshots, underlying_prices, underlying_prices, &now)?;
    map_snapshots_with_pricing_references(snapshots, Some(&pricing_references), dividend_yield)
}

pub fn map_snapshots_with_pricing_references(
    snapshots: &HashMap<String, Snapshot>,
    pricing_references: Option<&HashMap<String, OptionPricingReference>>,
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    ordered_snapshots(snapshots)
        .into_iter()
        .map(|(occ_symbol, snapshot)| {
            map_snapshot_with_pricing_reference(
                occ_symbol,
                snapshot,
                pricing_references.and_then(|references| references.get(occ_symbol)),
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

fn normalized_display_symbols<S: AsRef<str>>(symbols: &[S]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for symbol in symbols {
        let symbol = display_stock_symbol(symbol.as_ref());
        if !symbol.is_empty() && seen.insert(symbol.clone()) {
            normalized.push(symbol);
        }
    }
    normalized
}

fn latest_close_from_daily_bars(bars: &[alpaca_data::stocks::Bar]) -> Option<f64> {
    let mut latest: Option<(&str, f64)> = None;
    for bar in bars {
        let Some(close) = decimal_to_valid_price(bar.c) else {
            continue;
        };
        let timestamp = bar.t.as_deref().unwrap_or_default();
        if latest
            .map(|(latest_timestamp, _)| timestamp > latest_timestamp)
            .unwrap_or(true)
        {
            latest = Some((timestamp, close));
        }
    }
    latest.map(|(_, close)| close)
}

fn latest_close_prices_from_daily_bars(
    bars_by_symbol: HashMap<String, Vec<alpaca_data::stocks::Bar>>,
) -> HashMap<String, f64> {
    bars_by_symbol
        .into_iter()
        .filter_map(|(symbol, bars)| {
            latest_close_from_daily_bars(&bars).map(|price| (symbol, price))
        })
        .collect()
}

pub async fn latest_close_prices<S: AsRef<str>>(
    client: &Client,
    symbols: &[S],
) -> OptionResult<HashMap<String, f64>> {
    let symbols = normalized_display_symbols(symbols);
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }

    let start = range::add_days(&clock::today(), -14).unwrap_or_else(|_| clock::today());
    let response = client
        .stocks()
        .bars_all(BarsRequest {
            symbols,
            timeframe: TimeFrame::day_1(),
            start: Some(start),
            end: None,
            limit: Some(1000),
            adjustment: None,
            feed: None,
            sort: Some(Sort::Desc),
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .map_err(|error| {
            OptionError::new(
                "provider_latest_close_fetch_failed",
                format!("failed to load latest daily stock bars via alpaca-data: {error}"),
            )
        })?;

    Ok(latest_close_prices_from_daily_bars(response.bars))
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
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    let now = clock::now();
    let mut realtime_prices = HashMap::new();
    let mut close_prices = HashMap::new();

    if session::is_regular_session_at(&now) {
        realtime_prices = fetch_underlying_prices(client, snapshots, underlying_prices).await?;
    } else {
        let symbols = underlying_display_symbols(snapshots);
        close_prices = latest_close_prices(client, &symbols).await?;
    }

    let pricing_references = pricing_references_for_snapshots(
        snapshots,
        (!realtime_prices.is_empty()).then_some(&realtime_prices),
        (!close_prices.is_empty()).then_some(&close_prices),
        &now,
    )?;
    map_snapshots_with_pricing_references(
        snapshots,
        (!pricing_references.is_empty()).then_some(&pricing_references),
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
    let mapped_snapshots = map_live_snapshots(&snapshots, client, None, None).await?;
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
            option_right: None,
            strike: None,
            valuation_years: None,
        });
    }

    Ok(ResolvedOptionStratPositions {
        underlying_display_symbol: parsed.underlying_display_symbol,
        legs,
        positions,
    })
}

pub const SPEC_ADAPTER_API: &str = "spec/api/alpaca-adapter-api.md";

#[cfg(test)]
mod tests {
    use super::*;
    use alpaca_data::options::{
        Greeks as ProviderGreeks, Quote as ProviderOptionQuote, Snapshot as ProviderOptionSnapshot,
    };
    use alpaca_data::stocks::Bar as StockBar;
    use rust_decimal::Decimal;

    const OCC_SYMBOL: &str = "QQQ260602C00100000";

    fn decimal(value: f64, scale: u32) -> Decimal {
        alpaca_core::decimal::from_f64(value, scale)
    }

    fn option_price_for(spot: f64, evaluation_time: &str, volatility: f64) -> f64 {
        let contract = contract::parse_occ_symbol(OCC_SYMBOL).expect("test OCC should parse");
        let years = expiration::years(&contract.expiration_date, Some(evaluation_time), None)
            .max(MIN_TIME_YEARS);
        pricing::price_black_scholes(&alpaca_option::BlackScholesInput::new(
            spot,
            contract.strike,
            years,
            0.0,
            volatility,
            contract.option_right,
        ))
        .expect("test Black-Scholes price should compute")
    }

    fn option_snapshot(timestamp: &str, option_price: f64) -> ProviderOptionSnapshot {
        ProviderOptionSnapshot {
            latest_quote: Some(ProviderOptionQuote {
                t: Some(timestamp.to_owned()),
                bp: Some(decimal(option_price, 6)),
                ap: Some(decimal(option_price, 6)),
                ..ProviderOptionQuote::default()
            }),
            ..ProviderOptionSnapshot::default()
        }
    }

    fn snapshots_with_one(
        snapshot: ProviderOptionSnapshot,
    ) -> HashMap<String, ProviderOptionSnapshot> {
        HashMap::from([(OCC_SYMBOL.to_owned(), snapshot)])
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }

    #[test]
    fn regular_session_pricing_reference_uses_snapshot_time_and_realtime_spot() {
        let snapshots = snapshots_with_one(option_snapshot("2026-06-01 10:00:00", 1.25));
        let realtime_prices = HashMap::from([("QQQ".to_owned(), 101.25)]);
        let close_prices = HashMap::from([("QQQ".to_owned(), 99.50)]);

        let references = pricing_references_for_snapshots(
            &snapshots,
            Some(&realtime_prices),
            Some(&close_prices),
            "2026-06-01 10:15:00",
        )
        .expect("pricing references should resolve");

        let reference = references
            .get(OCC_SYMBOL)
            .expect("test contract should have a pricing reference");
        assert_eq!(reference.evaluation_time, "2026-06-01 10:00:00");
        assert_eq!(reference.underlying_price, Some(101.25));
    }

    #[test]
    fn non_regular_session_pricing_reference_uses_last_close_time_and_close_spot() {
        let snapshots = snapshots_with_one(option_snapshot("2026-06-01 19:59:00", 1.25));
        let realtime_prices = HashMap::from([("QQQ".to_owned(), 105.00)]);
        let close_prices = HashMap::from([("QQQ".to_owned(), 100.00)]);

        let references = pricing_references_for_snapshots(
            &snapshots,
            Some(&realtime_prices),
            Some(&close_prices),
            "2026-06-01 20:30:00",
        )
        .expect("pricing references should resolve");

        let reference = references
            .get(OCC_SYMBOL)
            .expect("test contract should have a pricing reference");
        assert_eq!(reference.evaluation_time, "2026-06-01 16:00:00");
        assert_eq!(reference.underlying_price, Some(100.00));
    }

    #[test]
    fn fallback_iv_uses_pricing_reference_time_and_spot() {
        let evaluation_time = "2026-06-01 16:00:00";
        let expected_iv = 0.37;
        let spot = 100.0;
        let option_price = option_price_for(spot, evaluation_time, expected_iv);
        let snapshot = option_snapshot("2026-06-01 20:00:00", option_price);
        let reference = OptionPricingReference {
            evaluation_time: evaluation_time.to_owned(),
            underlying_price: Some(spot),
        };

        let mapped =
            map_snapshot_with_pricing_reference(OCC_SYMBOL, &snapshot, Some(&reference), Some(0.0))
                .expect("snapshot should map");

        assert_eq!(mapped.underlying_price, Some(spot));
        assert_close(
            mapped
                .implied_volatility
                .expect("fallback IV should be inferred"),
            expected_iv,
            1e-5,
        );
    }

    #[test]
    fn provider_iv_is_preserved_when_valid() {
        let mut snapshot = option_snapshot("2026-06-01 10:00:00", 5.0);
        snapshot.implied_volatility = Some(decimal(0.42, 6));
        snapshot.greeks = Some(ProviderGreeks {
            delta: Some(decimal(0.5, 6)),
            gamma: Some(decimal(0.02, 6)),
            theta: Some(decimal(-0.04, 6)),
            vega: Some(decimal(0.12, 6)),
            rho: Some(decimal(0.03, 6)),
        });
        let reference = OptionPricingReference {
            evaluation_time: "2026-06-01 10:00:00".to_owned(),
            underlying_price: Some(100.0),
        };

        let mapped =
            map_snapshot_with_pricing_reference(OCC_SYMBOL, &snapshot, Some(&reference), Some(0.0))
                .expect("snapshot should map");

        assert_eq!(mapped.implied_volatility, Some(0.42));
        assert_eq!(mapped.greeks.as_ref().map(|greeks| greeks.delta), Some(0.5));
    }

    #[test]
    fn latest_close_prices_are_derived_only_from_recent_daily_bars() {
        let daily_bars = HashMap::from([
            (
                "QQQ".to_owned(),
                vec![
                    StockBar {
                        t: Some("2026-05-28T04:00:00Z".to_owned()),
                        c: Some(decimal(97.0, 2)),
                        ..StockBar::default()
                    },
                    StockBar {
                        t: Some("2026-05-29T04:00:00Z".to_owned()),
                        c: Some(decimal(99.0, 2)),
                        ..StockBar::default()
                    },
                ],
            ),
            (
                "SPY".to_owned(),
                vec![StockBar {
                    t: Some("2026-05-29T04:00:00Z".to_owned()),
                    c: Some(decimal(501.25, 2)),
                    ..StockBar::default()
                }],
            ),
        ]);

        let prices = latest_close_prices_from_daily_bars(daily_bars);

        assert_eq!(prices.get("QQQ").copied(), Some(99.0));
        assert_eq!(prices.get("SPY").copied(), Some(501.25));
    }
}
