use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::snapshot;
use crate::types::OptionSnapshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LiquidityOptionData {
    pub occ_symbol: String,
    pub option_right: String,
    pub strike: f64,
    pub expiration_date: String,
    pub dte: i32,
    pub delta: f64,
    pub spread_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub liquidity: Option<bool>,
    pub bid: f64,
    pub ask: f64,
    pub mark: f64,
    pub implied_volatility: f64,
}

impl LiquidityOptionData {
    pub fn from_snapshot(snapshot: &OptionSnapshot, dte: i32) -> Option<Self> {
        let contract = snapshot::contract(snapshot)?;

        Some(Self {
            occ_symbol: snapshot.occ_symbol().to_string(),
            option_right: contract.option_right.as_str().to_string(),
            strike: contract.strike,
            expiration_date: contract.expiration_date,
            dte,
            delta: snapshot.delta().abs(),
            spread_pct: snapshot::spread_pct(snapshot) * 100.0,
            liquidity: snapshot::liquidity(snapshot),
            bid: snapshot.bid(),
            ask: snapshot.ask(),
            mark: snapshot.price(),
            implied_volatility: snapshot.iv(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LiquidityStats {
    pub total_count: usize,
    pub avg_spread_pct: f64,
    pub median_spread_pct: f64,
    pub min_spread_pct: f64,
    pub max_spread_pct: f64,
    pub dte_range: (i32, i32),
    pub delta_range: (f64, f64),
}

impl Default for LiquidityStats {
    fn default() -> Self {
        Self {
            total_count: 0,
            avg_spread_pct: 0.0,
            median_spread_pct: 0.0,
            min_spread_pct: 0.0,
            max_spread_pct: 0.0,
            dte_range: (0, 0),
            delta_range: (0.0, 0.0),
        }
    }
}

impl LiquidityStats {
    pub fn from_options(options: &[LiquidityOptionData]) -> Self {
        if options.is_empty() {
            return Self::default();
        }

        let mut spreads: Vec<f64> = options.iter().map(|option| option.spread_pct).collect();
        spreads.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

        let avg_spread_pct = spreads.iter().sum::<f64>() / spreads.len() as f64;
        let median_spread_pct = if spreads.len() % 2 == 0 {
            let middle = spreads.len() / 2;
            (spreads[middle - 1] + spreads[middle]) / 2.0
        } else {
            spreads[spreads.len() / 2]
        };

        let min_dte = options.iter().map(|option| option.dte).min().unwrap_or(0);
        let max_dte = options.iter().map(|option| option.dte).max().unwrap_or(0);

        let min_delta = options
            .iter()
            .map(|option| option.delta)
            .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        let max_delta = options
            .iter()
            .map(|option| option.delta)
            .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        Self {
            total_count: options.len(),
            avg_spread_pct,
            median_spread_pct,
            min_spread_pct: spreads.first().copied().unwrap_or(0.0),
            max_spread_pct: spreads.last().copied().unwrap_or(0.0),
            dte_range: (min_dte, max_dte),
            delta_range: (min_delta, max_delta),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LiquidityData {
    pub underlying_symbol: String,
    pub as_of: String,
    pub underlying_price: f64,
    pub options: Vec<LiquidityOptionData>,
    pub stats: LiquidityStats,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LiquidityBatchResponse {
    pub results: HashMap<String, LiquidityData>,
}
