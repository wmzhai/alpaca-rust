use crate::error::{OptionError, OptionResult};
use crate::types::{PayoffLegInput, PositionSide};

const ROOT_EPSILON: f64 = 1e-9;

fn ensure_finite(code: &'static str, name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            code,
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn ensure_positive(code: &'static str, name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(code, name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            code,
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

fn validate_leg(leg: &PayoffLegInput) -> OptionResult<()> {
    ensure_positive("invalid_payoff_input", "strike", leg.strike)?;
    ensure_finite("invalid_payoff_input", "premium", leg.premium)?;
    Ok(())
}

fn leg_intrinsic(leg: &PayoffLegInput, underlying_price_at_expiry: f64) -> f64 {
    match leg.option_right.as_str() {
        "call" => (underlying_price_at_expiry - leg.strike).max(0.0),
        "put" => (leg.strike - underlying_price_at_expiry).max(0.0),
        _ => 0.0,
    }
}

fn signed_leg_payoff(leg: &PayoffLegInput, underlying_price_at_expiry: f64) -> f64 {
    let intrinsic = leg_intrinsic(leg, underlying_price_at_expiry);
    let quantity = leg.quantity as f64;
    match leg.position_side {
        PositionSide::Long => quantity * (intrinsic - leg.premium),
        PositionSide::Short => quantity * (leg.premium - intrinsic),
    }
}

fn leg_slope(leg: &PayoffLegInput, underlying_price_at_expiry: f64) -> f64 {
    let quantity = leg.quantity as f64;
    match (leg.option_right.as_str(), &leg.position_side) {
        ("call", PositionSide::Long) => {
            if underlying_price_at_expiry > leg.strike {
                quantity
            } else {
                0.0
            }
        }
        ("call", PositionSide::Short) => {
            if underlying_price_at_expiry > leg.strike {
                -quantity
            } else {
                0.0
            }
        }
        ("put", PositionSide::Long) => {
            if underlying_price_at_expiry < leg.strike {
                -quantity
            } else {
                0.0
            }
        }
        ("put", PositionSide::Short) => {
            if underlying_price_at_expiry < leg.strike {
                quantity
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn maybe_push_root(
    legs: &[PayoffLegInput],
    start: f64,
    end: f64,
    sample: f64,
    roots: &mut Vec<f64>,
) -> OptionResult<()> {
    let slope = legs.iter().map(|leg| leg_slope(leg, sample)).sum::<f64>();
    let value = strategy_payoff_at_expiry(legs, sample)?;
    if slope.abs() <= ROOT_EPSILON {
        return Ok(());
    }

    let root = sample - value / slope;
    if !root.is_finite() || root < 0.0 {
        return Ok(());
    }

    let start_ok = root + ROOT_EPSILON >= start;
    let end_ok = if end.is_finite() {
        root - ROOT_EPSILON <= end
    } else {
        true
    };
    if start_ok && end_ok {
        roots.push(root);
    }
    Ok(())
}

pub fn single_leg_payoff_at_expiry(
    option_right: &str,
    position_side: &str,
    strike: f64,
    premium: f64,
    quantity: u32,
    underlying_price_at_expiry: f64,
) -> OptionResult<f64> {
    let leg = PayoffLegInput::new(option_right, position_side, strike, premium, quantity)?;
    strategy_payoff_at_expiry(&[leg], underlying_price_at_expiry)
}

pub fn strategy_payoff_at_expiry(
    legs: &[PayoffLegInput],
    underlying_price_at_expiry: f64,
) -> OptionResult<f64> {
    ensure_finite(
        "invalid_payoff_input",
        "underlying_price_at_expiry",
        underlying_price_at_expiry,
    )?;
    if underlying_price_at_expiry < 0.0 {
        return Err(OptionError::new(
            "invalid_payoff_input",
            format!(
                "underlying_price_at_expiry must be non-negative: {underlying_price_at_expiry}"
            ),
        ));
    }

    let mut total = 0.0;
    for leg in legs {
        validate_leg(leg)?;
        total += signed_leg_payoff(leg, underlying_price_at_expiry);
    }
    Ok(total)
}

pub fn break_even_points(legs: &[PayoffLegInput]) -> OptionResult<Vec<f64>> {
    for leg in legs {
        validate_leg(leg)?;
    }
    if legs.is_empty() {
        return Ok(Vec::new());
    }

    let mut strikes = legs.iter().map(|leg| leg.strike).collect::<Vec<_>>();
    strikes.sort_by(|left, right| left.partial_cmp(right).unwrap());
    strikes.dedup_by(|left, right| (*left - *right).abs() <= ROOT_EPSILON);

    let mut roots = Vec::new();
    let mut interval_start = 0.0;
    for (index, boundary) in strikes.iter().enumerate() {
        let sample = if index == 0 {
            (boundary / 2.0).max(0.0)
        } else {
            (interval_start + boundary) / 2.0
        };
        maybe_push_root(legs, interval_start, *boundary, sample, &mut roots)?;
        interval_start = *boundary;
    }

    let tail_sample = interval_start + interval_start.max(1.0);
    maybe_push_root(legs, interval_start, f64::INFINITY, tail_sample, &mut roots)?;

    roots.sort_by(|left, right| left.partial_cmp(right).unwrap());
    roots.dedup_by(|left, right| (*left - *right).abs() <= 1e-7);
    Ok(roots)
}
