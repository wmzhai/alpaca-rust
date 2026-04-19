use serde::{Deserialize, Serialize};

use crate::error::{OptionError, OptionResult};
use crate::types::{OptionChain, OptionRight, OptionSnapshot};
use alpaca_time::expiration;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SnapshotFilter<'a> {
    pub occ_symbol: Option<&'a str>,
    pub expiration_date: Option<&'a str>,
    pub strike: Option<f64>,
    pub option_right: Option<&'a str>,
    pub strike_tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpirationDate {
    pub expiration_date: String,
    pub calendar_days: i64,
}

fn strike_tolerance(value: Option<f64>) -> f64 {
    match value {
        Some(parsed) if parsed >= 0.0 => parsed,
        _ => 0.01,
    }
}

fn matches_contract(snapshot: &OptionSnapshot, filter: SnapshotFilter<'_>) -> bool {
    if let Some(occ_symbol) = filter.occ_symbol {
        return snapshot.contract.occ_symbol.eq_ignore_ascii_case(occ_symbol.trim());
    }

    if let Some(expiration_date) = filter.expiration_date {
        if snapshot.contract.expiration_date != expiration_date {
            return false;
        }
    }

    if let Some(option_right) = filter.option_right {
        let Ok(option_right) = OptionRight::from_str(option_right) else {
            return false;
        };
        if snapshot.contract.option_right != option_right {
            return false;
        }
    }

    if let Some(strike) = filter.strike {
        if (snapshot.contract.strike - strike).abs() > strike_tolerance(filter.strike_tolerance) {
            return false;
        }
    }

    true
}

pub fn list_snapshots<'a>(chain: &'a OptionChain, filter: SnapshotFilter<'_>) -> Vec<&'a OptionSnapshot> {
    chain
        .snapshots
        .iter()
        .filter(|snapshot| matches_contract(snapshot, filter))
        .collect()
}

pub fn find_snapshot<'a>(chain: &'a OptionChain, filter: SnapshotFilter<'_>) -> Option<&'a OptionSnapshot> {
    chain
        .snapshots
        .iter()
        .find(|snapshot| matches_contract(snapshot, filter))
}

pub fn expiration_dates(
    chain: &OptionChain,
    option_right: Option<&str>,
    min_calendar_days: Option<i64>,
    max_calendar_days: Option<i64>,
    now: Option<&str>,
) -> OptionResult<Vec<ExpirationDate>> {
    let option_right = option_right.map(OptionRight::from_str).transpose()?;
    let mut results = Vec::new();

    for snapshot in &chain.snapshots {
        if let Some(expected) = &option_right {
            if &snapshot.contract.option_right != expected {
                continue;
            }
        }

        if results
            .iter()
            .any(|item: &ExpirationDate| item.expiration_date == snapshot.contract.expiration_date)
        {
            continue;
        }

        let calendar_days = expiration::calendar_days(&snapshot.contract.expiration_date, now)
            .map_err(|error| OptionError::new(error.code, error.message))?;
        if let Some(min_calendar_days) = min_calendar_days {
            if calendar_days < min_calendar_days {
                continue;
            }
        }
        if let Some(max_calendar_days) = max_calendar_days {
            if calendar_days > max_calendar_days {
                continue;
            }
        }

        results.push(ExpirationDate {
            expiration_date: snapshot.contract.expiration_date.clone(),
            calendar_days,
        });
    }

    results.sort_by(|left, right| {
        left.calendar_days
            .cmp(&right.calendar_days)
            .then_with(|| left.expiration_date.cmp(&right.expiration_date))
    });

    Ok(results)
}
