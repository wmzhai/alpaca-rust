use std::cmp::Ordering;

use alpaca_time::clock;

use crate::types::{
    MarketStructureAnalysis, MarketStructureAnalysisOptions, MarketStructureExposureMode,
    MarketStructureFilters, MarketStructureLevel, MarketStructureOptionRecord, OptionRight,
};

#[derive(Debug, Clone, Default)]
struct LevelAccumulator {
    strike: f64,
    call_open_interest: f64,
    put_open_interest: f64,
    call_gamma_exposure: f64,
    put_gamma_exposure: f64,
    call_volume: u64,
    put_volume: u64,
}

impl LevelAccumulator {
    fn into_level(self, labels: Vec<String>) -> MarketStructureLevel {
        let total_open_interest = self.call_open_interest + self.put_open_interest;
        let net_gamma_exposure = self.call_gamma_exposure + self.put_gamma_exposure;
        let absolute_gamma_exposure =
            self.call_gamma_exposure.abs() + self.put_gamma_exposure.abs();
        let total_volume = self.call_volume.saturating_add(self.put_volume);

        MarketStructureLevel {
            strike: self.strike,
            call_open_interest: self.call_open_interest,
            put_open_interest: self.put_open_interest,
            total_open_interest,
            call_gamma_exposure: self.call_gamma_exposure,
            put_gamma_exposure: self.put_gamma_exposure,
            net_gamma_exposure,
            absolute_gamma_exposure,
            call_volume: self.call_volume,
            put_volume: self.put_volume,
            total_volume,
            labels,
        }
    }
}

pub fn gamma_exposure(record: &MarketStructureOptionRecord, underlying_price: f64) -> Option<f64> {
    gamma_exposure_with_mode(
        record,
        underlying_price,
        MarketStructureExposureMode::GexProxy,
    )
}

pub fn gamma_exposure_with_mode(
    record: &MarketStructureOptionRecord,
    underlying_price: f64,
    mode: MarketStructureExposureMode,
) -> Option<f64> {
    gamma_exposure_from_gamma(record, underlying_price, record.gamma?, mode)
}

pub fn filter_market_structure_records(
    records: &[MarketStructureOptionRecord],
    filters: &MarketStructureFilters,
) -> Vec<MarketStructureOptionRecord> {
    records
        .iter()
        .filter(|record| matches_expiration(record, filters))
        .filter(|record| matches_option_right(record, filters))
        .filter(|record| matches_strike(record, filters))
        .filter(|record| matches_dte(record, filters))
        .filter(|record| matches_open_interest(record, filters))
        .cloned()
        .collect()
}

pub fn analyze_market_structure(
    records: &[MarketStructureOptionRecord],
) -> MarketStructureAnalysis {
    analyze_market_structure_with_options(records, &MarketStructureAnalysisOptions::default())
}

pub fn analyze_market_structure_with_options(
    records: &[MarketStructureOptionRecord],
    options: &MarketStructureAnalysisOptions,
) -> MarketStructureAnalysis {
    let records_count = records.len();
    let underlying_price = records
        .iter()
        .filter_map(|record| record.underlying_price.and_then(finite_positive))
        .next();
    let open_interest_count = records
        .iter()
        .filter(|record| record.open_interest.and_then(finite).is_some())
        .count();

    let mut warnings = Vec::new();
    if records.is_empty() {
        warnings.push("no_records".to_string());
    }
    if underlying_price.is_none() {
        warnings.push("missing_underlying_price".to_string());
    }
    if open_interest_count < records_count {
        warnings.push("incomplete_open_interest".to_string());
    }

    let mut accumulators = Vec::<LevelAccumulator>::new();
    if let Some(spot) = underlying_price {
        for record in records {
            let strike = match finite(record.strike) {
                Some(value) => value,
                None => continue,
            };
            let open_interest = record
                .open_interest
                .and_then(finite)
                .unwrap_or(0.0)
                .max(0.0);
            let volume = activity_volume(record);
            let exposure = gamma_exposure_with_mode(record, spot, options.mode).unwrap_or(0.0);

            let index = accumulators
                .iter()
                .position(|level| same_strike(level.strike, strike))
                .unwrap_or_else(|| {
                    accumulators.push(LevelAccumulator {
                        strike,
                        ..LevelAccumulator::default()
                    });
                    accumulators.len() - 1
                });
            let level = &mut accumulators[index];

            match record.option_right {
                OptionRight::Call => {
                    level.call_open_interest += open_interest;
                    level.call_gamma_exposure += exposure;
                    level.call_volume = level.call_volume.saturating_add(volume);
                }
                OptionRight::Put => {
                    level.put_open_interest += open_interest;
                    level.put_gamma_exposure += exposure;
                    level.put_volume = level.put_volume.saturating_add(volume);
                }
            }
        }
    }

    accumulators.sort_by(|left, right| compare_f64(left.strike, right.strike));

    let mut levels: Vec<MarketStructureLevel> = accumulators
        .into_iter()
        .map(|level| level.into_level(Vec::new()))
        .collect();

    if records_count > 0
        && (levels.is_empty()
            || !levels
                .iter()
                .any(|level| level.absolute_gamma_exposure > 0.0))
    {
        warnings.push("no_gamma_exposure_records".to_string());
    }

    let call_wall_strike = levels
        .iter()
        .filter(|level| level.call_gamma_exposure != 0.0)
        .max_by(|left, right| {
            compare_f64(
                left.call_gamma_exposure.abs(),
                right.call_gamma_exposure.abs(),
            )
        })
        .map(|level| level.strike)
        .or_else(|| {
            levels
                .iter()
                .filter(|level| level.call_open_interest > 0.0)
                .max_by(|left, right| {
                    compare_f64(left.call_open_interest, right.call_open_interest)
                })
                .map(|level| level.strike)
        });
    let put_wall_strike = levels
        .iter()
        .filter(|level| level.put_gamma_exposure != 0.0)
        .max_by(|left, right| {
            compare_f64(
                left.put_gamma_exposure.abs(),
                right.put_gamma_exposure.abs(),
            )
        })
        .map(|level| level.strike)
        .or_else(|| {
            levels
                .iter()
                .filter(|level| level.put_open_interest > 0.0)
                .max_by(|left, right| compare_f64(left.put_open_interest, right.put_open_interest))
                .map(|level| level.strike)
        });
    let absolute_wall_strike = levels
        .iter()
        .filter(|level| level.absolute_gamma_exposure > 0.0)
        .max_by(|left, right| {
            compare_f64(left.absolute_gamma_exposure, right.absolute_gamma_exposure)
        })
        .map(|level| level.strike)
        .or_else(|| {
            levels
                .iter()
                .filter(|level| level.total_open_interest > 0.0)
                .max_by(|left, right| {
                    compare_f64(left.total_open_interest, right.total_open_interest)
                })
                .map(|level| level.strike)
        });

    for level in &mut levels {
        if call_wall_strike
            .map(|strike| same_strike(level.strike, strike))
            .unwrap_or(false)
        {
            level.labels.push("call_wall".to_string());
        }
        if put_wall_strike
            .map(|strike| same_strike(level.strike, strike))
            .unwrap_or(false)
        {
            level.labels.push("put_wall".to_string());
        }
        if absolute_wall_strike
            .map(|strike| same_strike(level.strike, strike))
            .unwrap_or(false)
        {
            level
                .labels
                .push("absolute_gamma_exposure_wall".to_string());
        }
    }

    let call_wall = level_by_strike(&levels, call_wall_strike);
    let put_wall = level_by_strike(&levels, put_wall_strike);
    let absolute_gamma_exposure_wall = level_by_strike(&levels, absolute_wall_strike);
    let net_gamma_exposure = levels.iter().map(|level| level.net_gamma_exposure).sum();
    let absolute_gamma_exposure = levels
        .iter()
        .map(|level| level.absolute_gamma_exposure)
        .sum();
    let open_interest_coverage = if records_count == 0 {
        0.0
    } else {
        open_interest_count as f64 / records_count as f64
    };

    MarketStructureAnalysis {
        underlying_price,
        records_count,
        open_interest_coverage,
        call_wall,
        put_wall,
        absolute_gamma_exposure_wall,
        net_gamma_exposure,
        absolute_gamma_exposure,
        levels,
        warnings,
    }
}

fn matches_expiration(
    record: &MarketStructureOptionRecord,
    filters: &MarketStructureFilters,
) -> bool {
    filters
        .expiration_date
        .as_ref()
        .map(|expiration| record.expiration_date == *expiration)
        .unwrap_or(true)
}

fn matches_option_right(
    record: &MarketStructureOptionRecord,
    filters: &MarketStructureFilters,
) -> bool {
    filters
        .option_right
        .as_ref()
        .map(|right| record.option_right == *right)
        .unwrap_or(true)
}

fn matches_strike(record: &MarketStructureOptionRecord, filters: &MarketStructureFilters) -> bool {
    let Some(strike) = finite(record.strike) else {
        return false;
    };

    if filters
        .strike_price_gte
        .and_then(finite)
        .map(|min| strike < min)
        .unwrap_or(false)
    {
        return false;
    }
    if filters
        .strike_price_lte
        .and_then(finite)
        .map(|max| strike > max)
        .unwrap_or(false)
    {
        return false;
    }
    true
}

fn matches_dte(record: &MarketStructureOptionRecord, filters: &MarketStructureFilters) -> bool {
    if filters.dte_min.is_none() && filters.dte_max.is_none() {
        return true;
    }

    let Some(dte) = days_to_expiration(record) else {
        return false;
    };
    if filters
        .dte_min
        .and_then(finite)
        .map(|min| dte < min)
        .unwrap_or(false)
    {
        return false;
    }
    if filters
        .dte_max
        .and_then(finite)
        .map(|max| dte > max)
        .unwrap_or(false)
    {
        return false;
    }
    true
}

fn matches_open_interest(
    record: &MarketStructureOptionRecord,
    filters: &MarketStructureFilters,
) -> bool {
    if !filters.require_open_interest {
        return true;
    }
    record
        .open_interest
        .and_then(finite)
        .map(|open_interest| open_interest > 0.0)
        .unwrap_or(false)
}

fn days_to_expiration(record: &MarketStructureOptionRecord) -> Option<f64> {
    let as_of_date = record.as_of.get(0..10).unwrap_or(&record.as_of);
    clock::fractional_days_between(as_of_date, &record.expiration_date)
        .ok()
        .and_then(finite)
}

fn activity_volume(record: &MarketStructureOptionRecord) -> u64 {
    record
        .daily_volume
        .or(record.minute_volume)
        .or(record.latest_trade_size)
        .unwrap_or(0)
}

fn level_by_strike(
    levels: &[MarketStructureLevel],
    strike: Option<f64>,
) -> Option<MarketStructureLevel> {
    let strike = strike?;
    levels
        .iter()
        .find(|level| same_strike(level.strike, strike))
        .cloned()
}

fn gamma_exposure_from_gamma(
    record: &MarketStructureOptionRecord,
    spot: f64,
    gamma: f64,
    mode: MarketStructureExposureMode,
) -> Option<f64> {
    let gamma = finite(gamma)?;
    let open_interest = finite(record.open_interest?)?;
    let multiplier = finite(record.multiplier?)?;
    let spot = finite(spot)?;
    if open_interest < 0.0 || multiplier <= 0.0 || spot <= 0.0 {
        return None;
    }

    let exposure = gamma * open_interest * multiplier * spot * spot * 0.01;
    let signed_exposure = match (mode, record.option_right.clone()) {
        (MarketStructureExposureMode::GexProxy, OptionRight::Call)
        | (MarketStructureExposureMode::DealerView, OptionRight::Put) => exposure,
        (MarketStructureExposureMode::GexProxy, OptionRight::Put)
        | (MarketStructureExposureMode::DealerView, OptionRight::Call) => -exposure,
    };
    finite(signed_exposure)
}

fn finite(value: f64) -> Option<f64> {
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

fn finite_positive(value: f64) -> Option<f64> {
    finite(value).filter(|value| *value > 0.0)
}

fn same_strike(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-8
}

fn compare_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}
