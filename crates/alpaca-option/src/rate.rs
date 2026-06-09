use crate::DEFAULT_RISK_FREE_RATE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskFreeRatePoint {
    pub years: f64,
    pub rate: f64,
}

pub const DEFAULT_RISK_FREE_RATE_CURVE: [RiskFreeRatePoint; 14] = [
    RiskFreeRatePoint {
        years: 1.0 / 12.0,
        rate: 0.0370,
    },
    RiskFreeRatePoint {
        years: 1.5 / 12.0,
        rate: 0.0370,
    },
    RiskFreeRatePoint {
        years: 2.0 / 12.0,
        rate: 0.0371,
    },
    RiskFreeRatePoint {
        years: 3.0 / 12.0,
        rate: 0.0380,
    },
    RiskFreeRatePoint {
        years: 4.0 / 12.0,
        rate: 0.0379,
    },
    RiskFreeRatePoint {
        years: 6.0 / 12.0,
        rate: 0.0383,
    },
    RiskFreeRatePoint {
        years: 1.0,
        rate: 0.0385,
    },
    RiskFreeRatePoint {
        years: 2.0,
        rate: 0.0415,
    },
    RiskFreeRatePoint {
        years: 3.0,
        rate: 0.0421,
    },
    RiskFreeRatePoint {
        years: 5.0,
        rate: 0.0429,
    },
    RiskFreeRatePoint {
        years: 7.0,
        rate: 0.0442,
    },
    RiskFreeRatePoint {
        years: 10.0,
        rate: 0.0456,
    },
    RiskFreeRatePoint {
        years: 20.0,
        rate: 0.0505,
    },
    RiskFreeRatePoint {
        years: 30.0,
        rate: 0.0503,
    },
];

pub fn risk_free_rate_for_years(years: f64) -> f64 {
    if !years.is_finite() {
        return DEFAULT_RISK_FREE_RATE;
    }

    let first = DEFAULT_RISK_FREE_RATE_CURVE[0];
    if years <= first.years {
        return first.rate;
    }

    for window in DEFAULT_RISK_FREE_RATE_CURVE.windows(2) {
        let left = window[0];
        let right = window[1];
        if years <= right.years {
            let span = right.years - left.years;
            let weight = (years - left.years) / span;
            return left.rate + (right.rate - left.rate) * weight;
        }
    }

    DEFAULT_RISK_FREE_RATE_CURVE[DEFAULT_RISK_FREE_RATE_CURVE.len() - 1].rate
}
