use alpaca_core::float;
use alpaca_time::expiration;

use crate::contract;
use crate::execution_quote;
use crate::types::{OptionContract, OptionQuote, OptionSnapshot};

fn canonical_contract(contract_input: &OptionContract) -> Option<OptionContract> {
    if let Some(parsed) = contract::parse_occ_symbol(&contract_input.occ_symbol) {
        return Some(parsed);
    }

    let occ_symbol = contract::build_occ_symbol(
        &contract_input.underlying_symbol,
        &contract_input.expiration_date,
        contract_input.strike,
        contract_input.option_right.as_str(),
    )?;
    contract::parse_occ_symbol(&occ_symbol)
}

pub trait SnapshotLike {
    fn canonical_contract(&self) -> Option<OptionContract>;
    fn as_of(&self) -> &str;
    fn bid(&self) -> Option<f64>;
    fn ask(&self) -> Option<f64>;
    fn mark(&self) -> Option<f64>;
    fn last(&self) -> Option<f64>;
    fn delta(&self) -> Option<f64>;
}

impl<T: SnapshotLike + ?Sized> SnapshotLike for &T {
    fn canonical_contract(&self) -> Option<OptionContract> {
        (*self).canonical_contract()
    }

    fn as_of(&self) -> &str {
        (*self).as_of()
    }

    fn bid(&self) -> Option<f64> {
        (*self).bid()
    }

    fn ask(&self) -> Option<f64> {
        (*self).ask()
    }

    fn mark(&self) -> Option<f64> {
        (*self).mark()
    }

    fn last(&self) -> Option<f64> {
        (*self).last()
    }

    fn delta(&self) -> Option<f64> {
        (*self).delta()
    }
}

impl SnapshotLike for OptionSnapshot {
    fn canonical_contract(&self) -> Option<OptionContract> {
        canonical_contract(&self.contract)
    }

    fn as_of(&self) -> &str {
        &self.as_of
    }

    fn bid(&self) -> Option<f64> {
        self.quote.bid
    }

    fn ask(&self) -> Option<f64> {
        self.quote.ask
    }

    fn mark(&self) -> Option<f64> {
        self.quote.mark
    }

    fn last(&self) -> Option<f64> {
        self.quote.last
    }

    fn delta(&self) -> Option<f64> {
        self.greeks.as_ref().map(|greeks| greeks.delta)
    }
}

fn normalized_quote(snapshot: &impl SnapshotLike) -> OptionQuote {
    execution_quote::quote(&OptionQuote {
        bid: snapshot.bid(),
        ask: snapshot.ask(),
        mark: snapshot.mark(),
        last: snapshot.last(),
    })
}

pub fn contract(snapshot: &impl SnapshotLike) -> Option<OptionContract> {
    snapshot.canonical_contract()
}

pub fn spread(snapshot: &impl SnapshotLike) -> f64 {
    let normalized = normalized_quote(snapshot);
    float::round(
        normalized.ask.unwrap_or(0.0) - normalized.bid.unwrap_or(0.0),
        12,
    )
}

pub fn spread_pct(snapshot: &impl SnapshotLike) -> f64 {
    let price = normalized_quote(snapshot).mark.unwrap_or(0.0);
    if price.abs() <= 1e-10 {
        return 0.0;
    }

    spread(snapshot) / price
}

pub fn is_valid(snapshot: &impl SnapshotLike) -> bool {
    contract(snapshot).is_some() && !snapshot.as_of().trim().is_empty()
}

pub fn liquidity(snapshot: &impl SnapshotLike) -> Option<bool> {
    let price = normalized_quote(snapshot).mark.unwrap_or(0.0);
    if price.abs() <= 1e-10 {
        return None;
    }

    let contract = contract(snapshot)?;
    let calendar_days =
        expiration::calendar_days(&contract.expiration_date, Some(snapshot.as_of())).ok()?;

    let large_etfs = ["SPY", "QQQ", "IWM", "SMH", "GLD"];
    let is_etf = large_etfs.contains(&contract.underlying_symbol.as_str());
    let base_tolerance = if is_etf { 0.06 } else { 0.10 };
    let dte_factor = (1.0 + (calendar_days as f64 / 30.0) * 0.40).min(3.5_f64);
    let abs_delta = snapshot.delta().map(|delta| delta.abs()).unwrap_or(0.0);
    let delta_factor = if abs_delta < 0.3 {
        2.5
    } else if abs_delta > 0.7 {
        1.3
    } else {
        1.0
    };
    let tolerance = (base_tolerance * dte_factor * delta_factor).min(0.40_f64);

    Some(spread_pct(snapshot) <= tolerance)
}
