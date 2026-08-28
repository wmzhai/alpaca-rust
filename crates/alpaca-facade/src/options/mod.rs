#![forbid(unsafe_code)]

//! alpaca-facade
//!
//! Thin bridge helpers that map Alpaca option payloads into `alpaca-option`
//! core models.

use std::collections::HashMap;

use alpaca_core::decimal;
use alpaca_data::Client;
use alpaca_data::options::{OptionsFeed, Snapshot, SnapshotsRequest, ordered_snapshots};
use alpaca_data::stocks::display_stock_symbol;
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
use alpaca_time::session;
use rust_decimal::Decimal;
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
    #[serde(default)]
    pub iv_underlying_price: Option<Decimal>,
    pub underlying_price: Option<Decimal>,
    #[serde(default)]
    pub invert_iv: bool,
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
        delta: valid_float(greeks.delta)?,
        gamma: valid_float(greeks.gamma)?,
        vega: valid_float(greeks.vega)?,
        theta: valid_float(greeks.theta)?,
        rho: valid_float(greeks.rho)?,
    })
}

fn valid_float(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn valid_underlying_price(underlying_price: Option<f64>) -> Option<f64> {
    underlying_price.filter(|value| value.is_finite() && *value > 0.0)
}

fn valid_underlying_price_decimal(underlying_price: Option<Decimal>) -> Option<Decimal> {
    underlying_price.filter(|value| *value > Decimal::ZERO)
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

fn positive_quote_price(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn inversion_quote_price(quote: &OptionQuote, invert_iv: bool) -> Option<f64> {
    if invert_iv {
        match (
            positive_quote_price(quote.bid),
            positive_quote_price(quote.ask),
        ) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => positive_quote_price(quote.mark),
        }
    } else {
        positive_quote_price(quote.mark).or_else(|| positive_quote_price(quote.last))
    }
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

fn reference_spot(price: Option<Decimal>) -> Option<f64> {
    valid_underlying_price_decimal(price)
        .and_then(|price| price.to_f64())
        .and_then(|price| valid_underlying_price(Some(price)))
}

fn infer_iv(
    contract: &alpaca_option::OptionContract,
    option_price: f64,
    underlying_price: f64,
    years: f64,
    dividend_yield: f64,
) -> Option<f64> {
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
}

fn repaired_greeks_and_iv(
    contract: &alpaca_option::OptionContract,
    quote: &OptionQuote,
    provider_greeks: Option<Greeks>,
    provider_iv: Option<f64>,
    pricing_reference: Option<&OptionPricingReference>,
    dividend_yield: Option<f64>,
) -> (Option<Greeks>, Option<f64>) {
    let invert_iv = pricing_reference.is_some_and(|reference| reference.invert_iv);
    if !invert_iv && !snapshot_needs_repair(provider_greeks.as_ref(), provider_iv) {
        return (provider_greeks, valid_iv(provider_iv));
    }

    let fallback_greeks = (!invert_iv && !greeks_are_invalid(provider_greeks.as_ref()))
        .then_some(provider_greeks)
        .flatten();
    let fallback_iv = (!invert_iv).then(|| valid_iv(provider_iv)).flatten();
    let Some(pricing_reference) = pricing_reference else {
        return (fallback_greeks, fallback_iv);
    };
    let Some(iv_spot) = reference_spot(pricing_reference.iv_underlying_price) else {
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
        inversion_quote_price(quote, invert_iv)
            .and_then(|option_price| infer_iv(contract, option_price, iv_spot, years, dividend_yield))
    };

    let Some(implied_volatility) = implied_volatility else {
        return (fallback_greeks, fallback_iv);
    };

    let mut greeks = match pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput::new(
        iv_spot,
        contract.strike,
        years,
        dividend_yield,
        implied_volatility,
        contract.option_right.clone(),
    )) {
        Ok(greeks) => greeks,
        Err(_) => return (fallback_greeks, Some(implied_volatility)),
    };

    if let Some(option_price) = inversion_quote_price(quote, invert_iv) {
        capped_low_price_greeks(contract, option_price, iv_spot, &mut greeks);
    }

    (Some(greeks), Some(implied_volatility))
}

pub fn apply_optionstrat_premium_model(
    snapshot: &mut OptionSnapshot,
    premium_per_contract: Option<f64>,
    pricing_reference: Option<&OptionPricingReference>,
    dividend_yield: Option<f64>,
) -> OptionResult<()> {
    let Some(option_price) = premium_per_contract.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok(());
    };
    let Some(pricing_reference) = pricing_reference else {
        return Ok(());
    };
    let Some(iv_spot) = reference_spot(pricing_reference.iv_underlying_price).or_else(|| {
        (!pricing_reference.invert_iv)
            .then(|| reference_spot(pricing_reference.underlying_price))
            .flatten()
    }) else {
        return Ok(());
    };
    let contract = snapshot.contract.clone();
    let years = expiration::years(
        &contract.expiration_date,
        Some(&pricing_reference.evaluation_time),
        None,
    )
    .max(MIN_TIME_YEARS);
    let dividend_yield = dividend_yield.unwrap_or(DEFAULT_DIVIDEND_YIELD);
    let Some(implied_volatility) = infer_iv(
        &contract,
        option_price,
        iv_spot,
        years,
        dividend_yield,
    ) else {
        return Ok(());
    };

    snapshot.implied_volatility = Some(implied_volatility);
    snapshot.underlying_price = Some(iv_spot);
    if let Ok(greeks) = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput::new(
        iv_spot,
        contract.strike,
        years,
        dividend_yield,
        implied_volatility,
        contract.option_right,
    )) {
        snapshot.greeks = Some(greeks);
    }

    Ok(())
}

fn close_evaluation_time(now: &str) -> OptionResult<String> {
    calendar::last_completed_trading_date(Some(now))
        .map(|date| format!("{date} 16:00:00"))
        .map_err(|error| OptionError::new("invalid_pricing_time", error.to_string()))
}

fn pricing_reference_for_snapshot(
    snapshot: &Snapshot,
    latest_price: Option<Decimal>,
    iv_price: Option<Decimal>,
    now: &str,
) -> OptionResult<OptionPricingReference> {
    let invert_iv = !session::is_regular_session_at(now);
    let evaluation_time = if invert_iv {
        close_evaluation_time(now)?
    } else {
        snapshot_as_of_with_fallback(snapshot, now)
    };
    let latest_price = valid_underlying_price_decimal(latest_price);
    let iv_underlying_price = valid_underlying_price_decimal(iv_price).or_else(|| {
        if invert_iv {
            None
        } else {
            latest_price
        }
    });

    Ok(OptionPricingReference {
        evaluation_time,
        iv_underlying_price,
        underlying_price: latest_price,
        invert_iv,
    })
}

fn lookup_underlying_price(
    occ_symbol: &str,
    underlying_prices: Option<&HashMap<String, Decimal>>,
) -> Option<Decimal> {
    let underlying_prices = underlying_prices?;
    let contract = contract::parse_occ_symbol(occ_symbol)?;
    let display_symbol = display_stock_symbol(&contract.underlying_symbol);
    valid_underlying_price_decimal(
        underlying_prices
            .get(&contract.underlying_symbol)
            .copied()
            .or_else(|| underlying_prices.get(&display_symbol).copied()),
    )
}

pub fn pricing_references_for_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    latest_prices: Option<&HashMap<String, Decimal>>,
    iv_prices: Option<&HashMap<String, Decimal>>,
    now: &str,
) -> OptionResult<HashMap<String, OptionPricingReference>> {
    ordered_snapshots(snapshots)
        .into_iter()
        .map(|(occ_symbol, snapshot)| {
            let reference = pricing_reference_for_snapshot(
                snapshot,
                lookup_underlying_price(occ_symbol, latest_prices),
                lookup_underlying_price(occ_symbol, iv_prices),
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
    let provider_iv = snapshot.implied_volatility;
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
        underlying_price: pricing_reference.and_then(|reference| {
            reference_spot(reference.iv_underlying_price)
                .or_else(|| {
                    (!reference.invert_iv)
                        .then(|| reference_spot(reference.underlying_price))
                        .flatten()
                })
        }),
    })
}

pub fn map_snapshot(
    occ_symbol: &str,
    snapshot: &Snapshot,
    underlying_price: Option<Decimal>,
    dividend_yield: Option<f64>,
) -> OptionResult<OptionSnapshot> {
    let now = clock::now();
    let pricing_reference =
        pricing_reference_for_snapshot(snapshot, underlying_price, underlying_price, &now)?;
    map_snapshot_with_pricing_reference(
        occ_symbol,
        snapshot,
        Some(&pricing_reference),
        dividend_yield,
    )
}

pub fn map_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    underlying_prices: Option<&HashMap<String, Decimal>>,
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
    underlying_display_symbols(snapshots)
}

pub fn underlying_display_symbols(snapshots: &HashMap<String, Snapshot>) -> Vec<String> {
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

pub fn map_live_snapshots(
    snapshots: &HashMap<String, Snapshot>,
    latest_prices: Option<&HashMap<String, Decimal>>,
    iv_prices: Option<&HashMap<String, Decimal>>,
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    let now = clock::now();
    let iv_prices = iv_prices.or_else(|| {
        session::is_regular_session_at(&now)
            .then_some(latest_prices)
            .flatten()
    });
    let pricing_references =
        pricing_references_for_snapshots(snapshots, latest_prices, iv_prices, &now)?;
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
    let mapped_snapshots = map_live_snapshots(&snapshots, None, None, None)?;
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
        Trade as ProviderOptionTrade,
    };
    use rust_decimal::Decimal;

    const OCC_SYMBOL: &str = "QQQ260602C00100000";
    const SMH_OCC: &str = "SMH270617C00585000";

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

    fn pricing_reference(
        evaluation_time: &str,
        latest: f64,
        iv_spot: f64,
        invert_iv: bool,
    ) -> OptionPricingReference {
        OptionPricingReference {
            evaluation_time: evaluation_time.to_owned(),
            iv_underlying_price: Some(decimal(iv_spot, 2)),
            underlying_price: Some(decimal(latest, 2)),
            invert_iv,
        }
    }

    fn expected_iv(
        occ_symbol: &str,
        option_price: f64,
        spot: f64,
        evaluation_time: &str,
    ) -> f64 {
        let contract = contract::parse_occ_symbol(occ_symbol).expect("test OCC should parse");
        let years = expiration::years(&contract.expiration_date, Some(evaluation_time), None)
            .max(MIN_TIME_YEARS);
        infer_iv(&contract, option_price, spot, years, 0.0).expect("test IV should invert")
    }

    fn expected_greeks(
        occ_symbol: &str,
        spot: f64,
        evaluation_time: &str,
        implied_volatility: f64,
    ) -> Greeks {
        let contract = contract::parse_occ_symbol(occ_symbol).expect("test OCC should parse");
        pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput::new(
            spot,
            contract.strike,
            expiration::years(&contract.expiration_date, Some(evaluation_time), None)
                .max(MIN_TIME_YEARS),
            0.0,
            implied_volatility,
            contract.option_right,
        ))
        .expect("expected greeks should compute")
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
        let latest_prices = HashMap::from([("QQQ".to_owned(), decimal(101.25, 2))]);
        let close_prices = HashMap::from([("QQQ".to_owned(), decimal(99.50, 2))]);

        let references = pricing_references_for_snapshots(
            &snapshots,
            Some(&latest_prices),
            None,
            "2026-06-01 10:15:00",
        )
        .expect("pricing references should resolve");

        let reference = references
            .get(OCC_SYMBOL)
            .expect("test contract should have a pricing reference");
        assert_eq!(reference.evaluation_time, "2026-06-01 10:00:00");
        assert_eq!(reference.underlying_price, Some(decimal(101.25, 2)));
        assert_eq!(reference.iv_underlying_price, Some(decimal(101.25, 2)));
        assert!(!reference.invert_iv);

        let split = pricing_references_for_snapshots(
            &snapshots,
            Some(&latest_prices),
            Some(&close_prices),
            "2026-06-01 10:15:00",
        )
        .expect("pricing references should resolve");
        assert_eq!(
            split
                .get(OCC_SYMBOL)
                .expect("test contract should have a pricing reference")
                .iv_underlying_price,
            Some(decimal(99.50, 2))
        );
    }

    #[test]
    fn non_regular_session_keeps_latest_spot_and_close_iv_spot() {
        let snapshots = snapshots_with_one(option_snapshot("2026-06-01 19:59:00", 1.25));
        let latest_prices = HashMap::from([("QQQ".to_owned(), decimal(105.00, 2))]);
        let close_prices = HashMap::from([("QQQ".to_owned(), decimal(100.00, 2))]);

        let references = pricing_references_for_snapshots(
            &snapshots,
            Some(&latest_prices),
            Some(&close_prices),
            "2026-06-01 20:30:00",
        )
        .expect("pricing references should resolve");

        let reference = references
            .get(OCC_SYMBOL)
            .expect("test contract should have a pricing reference");
        assert_eq!(reference.evaluation_time, "2026-06-01 16:00:00");
        assert_eq!(reference.underlying_price, Some(decimal(105.00, 2)));
        assert_eq!(reference.iv_underlying_price, Some(decimal(100.00, 2)));
        assert!(reference.invert_iv);
    }

    #[test]
    fn fallback_iv_uses_pricing_reference_time_and_spot() {
        let evaluation_time = "2026-06-01 16:00:00";
        let expected_iv = 0.37;
        let spot = 100.0;
        let option_price = option_price_for(spot, evaluation_time, expected_iv);
        let snapshot = option_snapshot("2026-06-01 20:00:00", option_price);

        let mapped = map_snapshot_with_pricing_reference(
            OCC_SYMBOL,
            &snapshot,
            Some(&pricing_reference(evaluation_time, spot, spot, false)),
            Some(0.0),
        )
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
    fn optionstrat_premium_model_overrides_provider_iv_using_reference_close() {
        let evaluation_time = "2026-06-01 16:00:00";
        let expected_iv = 0.493;
        let close_spot = 100.0;
        let latest_spot = 105.0;
        let premium = option_price_for(close_spot, evaluation_time, expected_iv);
        let contract = contract::parse_occ_symbol(OCC_SYMBOL).expect("test OCC should parse");
        let mut snapshot = OptionSnapshot {
            as_of: "2026-06-01 20:00:00".to_owned(),
            contract: contract.clone(),
            quote: OptionQuote {
                bid: Some(premium),
                ask: Some(premium),
                mark: Some(premium),
                last: Some(premium),
            },
            greeks: None,
            implied_volatility: Some(0.12),
            underlying_price: None,
        };

        apply_optionstrat_premium_model(
            &mut snapshot,
            Some(premium),
            Some(&pricing_reference(
                evaluation_time,
                latest_spot,
                close_spot,
                true,
            )),
            Some(0.0),
        )
        .expect("premium model should apply");

        assert_eq!(snapshot.underlying_price, Some(close_spot));
        assert_close(
            snapshot
                .implied_volatility
                .expect("URL premium should infer IV"),
            expected_iv,
            1e-5,
        );
        let expected = expected_greeks(OCC_SYMBOL, close_spot, evaluation_time, expected_iv);
        assert_close(
            snapshot
                .greeks
                .as_ref()
                .expect("URL premium should repair greeks")
                .delta,
            expected.delta,
            1e-5,
        );
    }

    #[test]
    fn provider_iv_is_preserved_when_valid() {
        let mut snapshot = option_snapshot("2026-06-01 10:00:00", 5.0);
        snapshot.implied_volatility = Some(0.42);
        snapshot.greeks = Some(ProviderGreeks {
            delta: Some(0.5),
            gamma: Some(0.02),
            theta: Some(-0.04),
            vega: Some(0.12),
            rho: Some(0.03),
        });

        let mapped = map_snapshot_with_pricing_reference(
            OCC_SYMBOL,
            &snapshot,
            Some(&pricing_reference("2026-06-01 10:00:00", 100.0, 100.0, false)),
            Some(0.0),
        )
        .expect("snapshot should map");

        assert_eq!(mapped.implied_volatility, Some(0.42));
        assert_eq!(mapped.greeks.as_ref().map(|greeks| greeks.delta), Some(0.5));
    }

    #[test]
    fn off_hours_inverts_iv_and_greeks_from_close() {
        let evaluation_time = "2026-08-26 16:00:00";
        let close_spot = 555.77;
        let latest_spot = 575.0;
        let bid = 71.50;
        let ask = 75.50;
        let mid = (bid + ask) / 2.0;
        let last = 70.15;
        let snapshot = ProviderOptionSnapshot {
            latest_quote: Some(ProviderOptionQuote {
                t: Some("2026-08-26 15:59:59".to_owned()),
                bp: Some(decimal(bid, 2)),
                ap: Some(decimal(ask, 2)),
                ..ProviderOptionQuote::default()
            }),
            latest_trade: Some(ProviderOptionTrade {
                t: Some("2026-08-24 15:45:00".to_owned()),
                p: Some(decimal(last, 2)),
                ..ProviderOptionTrade::default()
            }),
            implied_volatility: Some(0.34),
            greeks: Some(ProviderGreeks {
                delta: Some(0.81),
                gamma: Some(0.002),
                theta: Some(-0.11),
                vega: Some(1.4),
                rho: Some(2.1),
            }),
            ..ProviderOptionSnapshot::default()
        };
        let close_iv = expected_iv(SMH_OCC, mid, close_spot, evaluation_time);
        let last_iv = expected_iv(SMH_OCC, last, close_spot, evaluation_time);
        let close_greeks = expected_greeks(SMH_OCC, close_spot, evaluation_time, close_iv);
        let latest_greeks = expected_greeks(SMH_OCC, latest_spot, evaluation_time, close_iv);

        let mapped = map_snapshot_with_pricing_reference(
            SMH_OCC,
            &snapshot,
            Some(&pricing_reference(
                evaluation_time,
                latest_spot,
                close_spot,
                true,
            )),
            Some(0.0),
        )
        .expect("snapshot should map");

        let mapped_iv = mapped
            .implied_volatility
            .expect("off-hours IV should be inferred from close");
        assert_close(mapped_iv, close_iv, 1e-5);
        assert!((mapped_iv - 0.34).abs() > 0.01);
        assert!((mapped_iv - last_iv).abs() > 1e-4);
        assert_eq!(mapped.underlying_price, Some(close_spot));
        assert_eq!(mapped.quote.bid, Some(bid));
        assert_eq!(mapped.quote.ask, Some(ask));
        assert_close(
            mapped.greeks.as_ref().expect("off-hours greeks").delta,
            close_greeks.delta,
            1e-5,
        );
        assert!((mapped.greeks.as_ref().unwrap().delta - latest_greeks.delta).abs() > 1e-4);
        assert_ne!(mapped.greeks.as_ref().unwrap().delta, 0.81);
    }

    #[test]
    fn off_hours_does_not_invert_iv_from_latest_when_close_is_missing() {
        let mut snapshot = option_snapshot("2026-08-26 15:59:59", 73.5);
        snapshot.implied_volatility = Some(0.34);
        snapshot.greeks = Some(ProviderGreeks {
            delta: Some(0.81),
            gamma: Some(0.002),
            theta: Some(-0.11),
            vega: Some(1.4),
            rho: Some(2.1),
        });
        let reference = OptionPricingReference {
            evaluation_time: "2026-08-26 16:00:00".to_owned(),
            iv_underlying_price: None,
            underlying_price: Some(decimal(575.0, 2)),
            invert_iv: true,
        };

        let mapped =
            map_snapshot_with_pricing_reference(SMH_OCC, &snapshot, Some(&reference), Some(0.0))
                .expect("snapshot should map");

        assert_eq!(mapped.implied_volatility, None);
        assert_eq!(mapped.greeks, None);
        assert_eq!(mapped.underlying_price, None);
    }
}
