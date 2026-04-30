#![forbid(unsafe_code)]

//! alpaca-facade
//!
//! Thin bridge helpers that map Alpaca option payloads into `alpaca-option`
//! core models.

use std::collections::HashMap;

use alpaca_core::decimal;
use alpaca_data::Client;
use alpaca_data::options::{
    OptionsFeed, Snapshot, SnapshotsRequest, ordered_snapshots,
};
use alpaca_data::stocks::{DataFeed, display_stock_symbol};
use alpaca_option::contract;
use alpaca_option::pricing;
use alpaca_option::url;
use alpaca_option::{
    Greeks, OptionError, OptionPosition, OptionQuote, OptionResult, OptionSnapshot,
    StrategyLegInput,
};
use alpaca_time::clock;
use alpaca_time::expiration;
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

fn decimal_to_f64(value: Option<rust_decimal::Decimal>) -> Option<f64> {
    value.and_then(|number| number.to_f64())
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

pub fn map_snapshot(
    occ_symbol: &str,
    snapshot: &Snapshot,
    underlying_price: Option<f64>,
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
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    ordered_snapshots(snapshots)
        .into_iter()
        .map(|(occ_symbol, snapshot)| {
            map_snapshot(
                occ_symbol,
                snapshot,
                lookup_underlying_price(occ_symbol, underlying_prices),
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
    dividend_yield: Option<f64>,
) -> OptionResult<Vec<OptionSnapshot>> {
    let underlying_prices = fetch_underlying_prices(client, snapshots, underlying_prices).await?;
    map_snapshots(
        snapshots,
        (!underlying_prices.is_empty()).then_some(&underlying_prices),
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
        });
    }

    Ok(ResolvedOptionStratPositions {
        underlying_display_symbol: parsed.underlying_display_symbol,
        legs,
        positions,
    })
}

pub const SPEC_ADAPTER_API: &str = "spec/api/alpaca-adapter-api.md";
