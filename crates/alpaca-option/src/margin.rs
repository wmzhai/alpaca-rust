//! Reg-T style collateral for the option structures that quote a finite margin.
//! Other structures return `None`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollateralLeg {
    pub long: bool,
    pub call: bool,
    pub strike: f64,
    pub basis: f64,
    pub size: f64,
}

fn requirement(leg: &CollateralLeg, spot: f64) -> f64 {
    let out_of_money = if leg.call {
        (leg.strike - spot).max(0.0)
    } else {
        (spot - leg.strike).max(0.0)
    };
    (leg.basis + 0.2 * spot - out_of_money).max(leg.basis + 0.1 * leg.strike)
}

fn naked(legs: &[CollateralLeg], spot: f64) -> f64 {
    legs.iter()
        .map(|leg| (requirement(leg, spot) - leg.basis) * 100.0 * leg.size)
        .sum()
}

/// Dollar collateral. `None` when this structure has no quoted margin.
pub fn strategy_collateral(kind: &str, legs: &[CollateralLeg], spot: f64) -> Option<f64> {
    if legs.is_empty() {
        return None;
    }
    match kind {
        "short-call" | "short-calls" | "short-put" | "short-puts" => Some(naked(legs, spot)),
        "cash-secured-put" => {
            let leg = &legs[0];
            Some(leg.strike * 100.0 * leg.size)
        }
        "bear-call-spread" | "bull-put-spread" => {
            let long = legs.iter().find(|leg| leg.long)?;
            let short = legs.iter().find(|leg| !leg.long)?;
            Some((long.strike * 100.0 * long.size - short.strike * 100.0 * short.size).abs())
        }
        "short-straddle" | "short-strangle" => {
            let call = legs.iter().find(|leg| leg.call)?;
            let put = legs.iter().find(|leg| !leg.call)?;
            let call_requirement = requirement(call, spot) * call.size;
            let put_requirement = requirement(put, spot) * put.size;
            let premium = legs.iter().map(|leg| leg.basis * leg.size).sum::<f64>();
            let larger = if call_requirement > put_requirement {
                call_requirement + put.basis * put.size
            } else {
                put_requirement + call.basis * call.size
            };
            Some((larger - premium) * 100.0)
        }
        "call-ratio-spread" | "put-ratio-spread" => {
            let long = legs.iter().find(|leg| leg.long)?;
            let short = legs.iter().find(|leg| !leg.long)?;
            Some((requirement(short, spot) - short.basis) * (short.size - long.size) * 100.0)
        }
        "long-synthetic-future" | "short-synthetic-future" | "long-combo" | "short-combo" => {
            let short = legs.iter().find(|leg| !leg.long)?;
            Some((requirement(short, spot) - short.basis) * 100.0 * short.size)
        }
        _ => None,
    }
}
