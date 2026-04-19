use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::pricing;
use crate::types::{
    OptionRight, PayoffLegInput, PositionSide, StrategyBreakEvenInput, StrategyPnlInput,
    StrategyValuationPosition,
};
use alpaca_time::clock;
use alpaca_time::expiration;

const ROOT_EPSILON: f64 = 1e-9;
const CONTRACT_MULTIPLIER: f64 = 100.0;

#[derive(Debug, Clone)]
struct PreparedStrategyLeg {
    option_right: OptionRight,
    strike: f64,
    quantity: f64,
    years: f64,
    implied_volatility: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct StrategyValuationContext {
    prepared: Vec<PreparedStrategyLeg>,
    entry_cost: f64,
    rate: f64,
    dividend_yield: f64,
}

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

fn validate_strategy_position(position: &StrategyValuationPosition) -> OptionResult<()> {
    ensure_positive(
        "invalid_strategy_payoff_input",
        "contract.strike",
        position.contract.strike,
    )?;
    if position.quantity == 0 {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            "quantity must not be zero",
        ));
    }
    if let Some(avg_entry_price) = position.avg_entry_price {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "avg_entry_price",
            avg_entry_price,
        )?;
    }
    if let Some(implied_volatility) = position.implied_volatility {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "implied_volatility",
            implied_volatility,
        )?;
    }
    if let Some(mark_price) = position.mark_price {
        ensure_finite("invalid_strategy_payoff_input", "mark_price", mark_price)?;
    }
    if let Some(reference_underlying_price) = position.reference_underlying_price {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "reference_underlying_price",
            reference_underlying_price,
        )?;
    }
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

fn valuation_years(expiration_date: &str, evaluation_time: &str) -> OptionResult<f64> {
    expiration::close(expiration_date).map_err(|error| {
        OptionError::new(
            "invalid_strategy_payoff_input",
            format!("invalid expiration context for {expiration_date}: {error}"),
        )
    })?;
    Ok(expiration::years(
        expiration_date,
        Some(evaluation_time),
        None,
    ))
}

fn strategy_entry_cost(
    positions: &[StrategyValuationPosition],
    entry_cost: Option<f64>,
) -> OptionResult<f64> {
    if let Some(entry_cost) = entry_cost {
        ensure_finite("invalid_strategy_payoff_input", "entry_cost", entry_cost)?;
        return Ok(entry_cost);
    }

    let mut total = 0.0;
    for position in positions {
        let avg_entry_price = position.avg_entry_price.ok_or_else(|| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                "entry_cost is required when avg_entry_price is missing",
            )
        })?;
        total += avg_entry_price * f64::from(position.quantity) * CONTRACT_MULTIPLIER;
    }
    Ok(total)
}

fn prepare_strategy_context(
    positions: &[StrategyValuationPosition],
    evaluation_time: &str,
    entry_cost: Option<f64>,
    dividend_yield: Option<f64>,
    long_volatility_shift: Option<f64>,
) -> OptionResult<(Vec<PreparedStrategyLeg>, f64, f64)> {
    let dividend_yield = dividend_yield.unwrap_or(0.0);
    ensure_finite(
        "invalid_strategy_payoff_input",
        "dividend_yield",
        dividend_yield,
    )?;
    if let Some(long_volatility_shift) = long_volatility_shift {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "long_volatility_shift",
            long_volatility_shift,
        )?;
    }

    clock::parse_timestamp(evaluation_time).map_err(|error| {
        OptionError::new(
            "invalid_strategy_payoff_input",
            format!("invalid evaluation_time: {error}"),
        )
    })?;

    let entry_cost = strategy_entry_cost(positions, entry_cost)?;
    let mut prepared = Vec::with_capacity(positions.len());
    for position in positions {
        validate_strategy_position(position)?;
        let quantity = f64::from(position.quantity);
        let years = valuation_years(&position.contract.expiration_date, evaluation_time)?;
        let implied_volatility = if years <= 0.0 {
            None
        } else {
            let mut implied_volatility = position.implied_volatility.ok_or_else(|| {
                OptionError::new(
                    "invalid_strategy_payoff_input",
                    format!(
                        "implied_volatility is required before expiration: {}",
                        position.contract.occ_symbol
                    ),
                )
            })?;
            if quantity > 0.0 {
                implied_volatility += long_volatility_shift.unwrap_or(0.0);
            }
            ensure_positive(
                "invalid_strategy_payoff_input",
                "implied_volatility",
                implied_volatility,
            )?;
            Some(implied_volatility)
        };

        prepared.push(PreparedStrategyLeg {
            option_right: position.contract.option_right.clone(),
            strike: position.contract.strike,
            quantity,
            years,
            implied_volatility,
        });
    }

    Ok((prepared, entry_cost, dividend_yield))
}

fn strategy_mark_value_prepared(
    positions: &[PreparedStrategyLeg],
    underlying_price: f64,
    rate: f64,
    dividend_yield: f64,
) -> OptionResult<f64> {
    ensure_finite(
        "invalid_strategy_payoff_input",
        "underlying_price",
        underlying_price,
    )?;
    if underlying_price < 0.0 {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            format!("underlying_price must be non-negative: {underlying_price}"),
        ));
    }
    ensure_finite("invalid_strategy_payoff_input", "rate", rate)?;

    let mut total = 0.0;
    for position in positions {
        let option_value = if position.years <= 0.0 {
            pricing::intrinsic_value(
                underlying_price,
                position.strike,
                position.option_right.as_str(),
            )?
        } else {
            pricing::price_black_scholes(&crate::BlackScholesInput {
                spot: underlying_price,
                strike: position.strike,
                years: position.years,
                rate,
                dividend_yield,
                volatility: position
                    .implied_volatility
                    .expect("prepared implied_volatility"),
                option_right: position.option_right.clone(),
            })?
        };
        total += option_value * position.quantity * CONTRACT_MULTIPLIER;
    }
    Ok(total)
}

fn push_unique_root(roots: &mut Vec<f64>, root: f64, tolerance: f64) {
    if roots
        .iter()
        .any(|existing| (*existing - root).abs() <= tolerance)
    {
        return;
    }
    roots.push(root);
}

fn validate_break_even_params(
    lower_bound: f64,
    upper_bound: f64,
    scan_step: Option<f64>,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<(f64, f64, usize)> {
    ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
    ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
    if lower_bound >= upper_bound {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            format!("lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"),
        ));
    }

    let tolerance = tolerance.unwrap_or(1e-9);
    ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
    let scan_step = scan_step.unwrap_or(1.0);
    ensure_positive("invalid_strategy_payoff_input", "scan_step", scan_step)?;
    let max_iterations = max_iterations.unwrap_or(100);
    if max_iterations == 0 {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            "max_iterations must be greater than zero",
        ));
    }

    Ok((tolerance, scan_step, max_iterations))
}

impl StrategyValuationContext {
    pub fn prepare(
        positions: &[StrategyValuationPosition],
        evaluation_time: &str,
        entry_cost: Option<f64>,
        rate: f64,
        dividend_yield: Option<f64>,
        long_volatility_shift: Option<f64>,
    ) -> OptionResult<Self> {
        ensure_finite("invalid_strategy_payoff_input", "rate", rate)?;
        let (prepared, entry_cost, dividend_yield) = prepare_strategy_context(
            positions,
            evaluation_time,
            entry_cost,
            dividend_yield,
            long_volatility_shift,
        )?;
        Ok(Self {
            prepared,
            entry_cost,
            rate,
            dividend_yield,
        })
    }

    pub fn pnl_at(&self, underlying_price: f64) -> OptionResult<f64> {
        Ok(strategy_mark_value_prepared(
            &self.prepared,
            underlying_price,
            self.rate,
            self.dividend_yield,
        )? - self.entry_cost)
    }

    pub fn break_even_points(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        scan_step: Option<f64>,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<Vec<f64>> {
        let (tolerance, scan_step, max_iterations) = validate_break_even_params(
            lower_bound,
            upper_bound,
            scan_step,
            tolerance,
            max_iterations,
        )?;

        let mut roots = Vec::new();
        let mut previous_spot = lower_bound;
        let mut previous_value = self.pnl_at(previous_spot)?;
        if previous_value.abs() <= tolerance {
            push_unique_root(&mut roots, previous_spot, tolerance * 10.0);
        }

        let mut current_spot = (previous_spot + scan_step).min(upper_bound);
        while current_spot <= upper_bound {
            let current_value = self.pnl_at(current_spot)?;
            let root = if current_value.abs() <= tolerance {
                Some(current_spot)
            } else if previous_value.abs() <= tolerance {
                Some(previous_spot)
            } else if previous_value.signum() != current_value.signum() {
                Some(numeric::refine_bracketed_root(
                    previous_spot,
                    current_spot,
                    |spot| {
                        Ok(strategy_mark_value_prepared(
                            &self.prepared,
                            spot,
                            self.rate,
                            self.dividend_yield,
                        )
                        .expect("validated strategy valuation context")
                            - self.entry_cost)
                    },
                    Some(tolerance),
                    Some(max_iterations),
                )?)
            } else {
                None
            };

            if let Some(root) = root {
                push_unique_root(&mut roots, root, tolerance * 10.0);
            }

            if current_spot >= upper_bound {
                break;
            }
            previous_spot = current_spot;
            previous_value = current_value;
            current_spot = (current_spot + scan_step).min(upper_bound);
        }

        roots.sort_by(|left, right| left.partial_cmp(right).unwrap());
        Ok(roots)
    }
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

pub fn strategy_pnl(input: &StrategyPnlInput) -> OptionResult<f64> {
    StrategyValuationContext::prepare(
        &input.positions,
        &input.evaluation_time,
        input.entry_cost,
        input.rate,
        input.dividend_yield,
        input.long_volatility_shift,
    )?
    .pnl_at(input.underlying_price)
}

pub fn strategy_break_even_points(input: &StrategyBreakEvenInput) -> OptionResult<Vec<f64>> {
    StrategyValuationContext::prepare(
        &input.positions,
        &input.evaluation_time,
        input.entry_cost,
        input.rate,
        input.dividend_yield,
        input.long_volatility_shift,
    )?
    .break_even_points(
        input.lower_bound,
        input.upper_bound,
        input.scan_step,
        input.tolerance,
        input.max_iterations,
    )
}
