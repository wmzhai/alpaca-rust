use std::sync::{Arc, OnceLock, RwLock};

use crate::DEFAULT_RISK_FREE_RATE;
use crate::error::{OptionError, OptionResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskFreeRatePoint {
    pub years: f64,
    pub rate: f64,
}

/// Treasury par yield curve for 2026-09-30, the latest complete official curve.
/// Live pricing replaces this after Universe publishes a newer row.
pub const DEFAULT_RISK_FREE_RATE_CURVE: [RiskFreeRatePoint; 14] = [
    RiskFreeRatePoint {
        years: 1.0 / 12.0,
        rate: 0.0402,
    },
    RiskFreeRatePoint {
        years: 1.5 / 12.0,
        rate: 0.0413,
    },
    RiskFreeRatePoint {
        years: 2.0 / 12.0,
        rate: 0.0416,
    },
    RiskFreeRatePoint {
        years: 3.0 / 12.0,
        rate: 0.0420,
    },
    RiskFreeRatePoint {
        years: 4.0 / 12.0,
        rate: 0.0429,
    },
    RiskFreeRatePoint {
        years: 6.0 / 12.0,
        rate: 0.0433,
    },
    RiskFreeRatePoint {
        years: 1.0,
        rate: 0.0454,
    },
    RiskFreeRatePoint {
        years: 2.0,
        rate: 0.0488,
    },
    RiskFreeRatePoint {
        years: 3.0,
        rate: 0.0500,
    },
    RiskFreeRatePoint {
        years: 5.0,
        rate: 0.0509,
    },
    RiskFreeRatePoint {
        years: 7.0,
        rate: 0.0519,
    },
    RiskFreeRatePoint {
        years: 10.0,
        rate: 0.0529,
    },
    RiskFreeRatePoint {
        years: 20.0,
        rate: 0.0568,
    },
    RiskFreeRatePoint {
        years: 30.0,
        rate: 0.0564,
    },
];

static INSTALLED_CURVE: RwLock<Option<Arc<[RiskFreeRatePoint]>>> = RwLock::new(None);

fn default_curve() -> Arc<[RiskFreeRatePoint]> {
    static DEFAULT: OnceLock<Arc<[RiskFreeRatePoint]>> = OnceLock::new();
    DEFAULT
        .get_or_init(|| Arc::<[RiskFreeRatePoint]>::from(DEFAULT_RISK_FREE_RATE_CURVE.as_slice()))
        .clone()
}

/// Curve used by new pricing work. An installed curve replaces the compiled fallback.
pub fn active_curve() -> Arc<[RiskFreeRatePoint]> {
    let guard = INSTALLED_CURVE
        .read()
        .unwrap_or_else(|error| error.into_inner());
    guard.clone().unwrap_or_else(default_curve)
}

pub fn install_risk_free_rate_curve(points: &[RiskFreeRatePoint]) -> OptionResult<()> {
    if points.len() < 2 {
        return Err(OptionError::new(
            "invalid_risk_free_rate_curve",
            "risk-free curve must contain at least two points",
        ));
    }

    let mut previous_years = f64::NEG_INFINITY;
    for point in points {
        if !point.years.is_finite() || point.years <= previous_years {
            return Err(OptionError::new(
                "invalid_risk_free_rate_curve",
                format!(
                    "curve years must be finite and strictly increasing: {}",
                    point.years
                ),
            ));
        }
        if !point.rate.is_finite() || point.rate <= 0.0 || point.rate >= 1.0 {
            return Err(OptionError::new(
                "invalid_risk_free_rate_curve",
                format!("curve rate must be in (0, 1): {}", point.rate),
            ));
        }
        previous_years = point.years;
    }

    let mut guard = INSTALLED_CURVE
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *guard = Some(Arc::<[RiskFreeRatePoint]>::from(points));
    Ok(())
}

pub fn clear_risk_free_rate_curve() {
    let mut guard = INSTALLED_CURVE
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *guard = None;
}

pub fn risk_free_rate_for_years(years: f64) -> f64 {
    rate_on_curve(&active_curve(), years)
}

pub fn rate_on_curve(curve: &[RiskFreeRatePoint], years: f64) -> f64 {
    if !years.is_finite() || curve.is_empty() {
        return DEFAULT_RISK_FREE_RATE;
    }

    let first = curve[0];
    if years <= first.years {
        return first.rate;
    }

    for window in curve.windows(2) {
        let left = window[0];
        let right = window[1];
        if years <= right.years {
            let span = right.years - left.years;
            if span <= 0.0 {
                return right.rate;
            }
            let weight = (years - left.years) / span;
            return left.rate + (right.rate - left.rate) * weight;
        }
    }

    curve[curve.len() - 1].rate
}
