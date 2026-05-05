use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::pricing;
use crate::types::{
    Greeks, OptionPosition, OptionRight, OptionStrategyCurvePoint, OptionStrategyInput,
    StrategyBreakEvenInput, StrategyPnlInput, StrategyValuationPosition,
};
use crate::DEFAULT_RISK_FREE_RATE;
use alpaca_time::clock;
use alpaca_time::expiration;

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
pub struct OptionStrategy {
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

fn validate_strategy_quantity(strategy_quantity: f64) -> OptionResult<f64> {
    ensure_positive(
        "invalid_strategy_payoff_input",
        "strategy_quantity",
        strategy_quantity,
    )?;
    Ok(strategy_quantity)
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

fn validate_break_even_bracket_params(
    lower_bound: f64,
    upper_bound: f64,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<(f64, usize)> {
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
    let max_iterations = max_iterations.unwrap_or(100);
    if max_iterations == 0 {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            "max_iterations must be greater than zero",
        ));
    }

    Ok((tolerance, max_iterations))
}

impl OptionStrategy {
    pub fn expiration_time(positions: &[StrategyValuationPosition]) -> OptionResult<String> {
        let expiration_date = positions
            .iter()
            .map(|position| position.contract.expiration_date.trim())
            .filter(|expiration_date| !expiration_date.is_empty())
            .min()
            .ok_or_else(|| {
                OptionError::new(
                    "invalid_strategy_payoff_input",
                    "at least one position with expiration_date is required",
                )
            })?;

        expiration::close(expiration_date).map_err(|error| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!("invalid expiration context for {expiration_date}: {error}"),
            )
        })
    }

    pub fn from_input(input: &OptionStrategyInput) -> OptionResult<Self> {
        let evaluation_time = input
            .evaluation_time
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .map(Ok)
            .unwrap_or_else(|| Self::expiration_time(&input.positions))?;

        Self::prepare_with_rate(
            &input.positions,
            &evaluation_time,
            input.entry_cost,
            input.rate.unwrap_or(DEFAULT_RISK_FREE_RATE),
            input.dividend_yield,
            input.long_volatility_shift,
        )
    }

    pub fn prepare(
        positions: &[StrategyValuationPosition],
        evaluation_time: &str,
        entry_cost: Option<f64>,
        dividend_yield: Option<f64>,
        long_volatility_shift: Option<f64>,
    ) -> OptionResult<Self> {
        Self::prepare_with_rate(
            positions,
            evaluation_time,
            entry_cost,
            DEFAULT_RISK_FREE_RATE,
            dividend_yield,
            long_volatility_shift,
        )
    }

    pub fn prepare_with_rate(
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
        Ok(self.mark_value_at(underlying_price)? - self.entry_cost)
    }

    pub fn mark_value_at(&self, underlying_price: f64) -> OptionResult<f64> {
        strategy_mark_value_prepared(
            &self.prepared,
            underlying_price,
            self.rate,
            self.dividend_yield,
        )
    }

    pub fn sample_curve(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        step: f64,
    ) -> OptionResult<Vec<OptionStrategyCurvePoint>> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound < 0.0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!("lower_bound must be non-negative: {lower_bound}"),
            ));
        }
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }
        ensure_positive("invalid_strategy_payoff_input", "step", step)?;

        let mut points = Vec::new();
        let mut underlying_price = lower_bound;
        loop {
            let mark_value = self.mark_value_at(underlying_price)?;
            points.push(OptionStrategyCurvePoint {
                underlying_price,
                mark_value,
                pnl: mark_value - self.entry_cost,
            });

            if underlying_price >= upper_bound {
                break;
            }

            let next = (underlying_price + step).min(upper_bound);
            if next <= underlying_price {
                return Err(OptionError::new(
                    "invalid_strategy_payoff_input",
                    "step did not advance strategy curve scan",
                ));
            }
            underlying_price = next;
        }

        Ok(points)
    }

    pub fn break_even_between(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<Option<f64>> {
        let (tolerance, max_iterations) = validate_break_even_bracket_params(
            lower_bound,
            upper_bound,
            tolerance,
            max_iterations,
        )?;

        let lower_value = self.pnl_at(lower_bound)?;
        if lower_value.abs() <= tolerance {
            return Ok(Some(lower_bound));
        }

        let upper_value = self.pnl_at(upper_bound)?;
        if upper_value.abs() <= tolerance {
            return Ok(Some(upper_bound));
        }

        if lower_value.signum() == upper_value.signum() {
            return Ok(None);
        }

        numeric::refine_bracketed_root(
            lower_bound,
            upper_bound,
            |spot| self.pnl_at(spot),
            Some(tolerance),
            Some(max_iterations),
        )
        .map(Some)
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
                    |spot| self.pnl_at(spot),
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

    pub fn aggregate_snapshot_greeks(
        positions: &[OptionPosition],
        strategy_quantity: f64,
    ) -> OptionResult<Greeks> {
        let strategy_quantity = validate_strategy_quantity(strategy_quantity)?;
        let mut total = Greeks::default();

        for position in positions {
            let quantity = f64::from(position.qty);
            let greeks = position.snapshot.greeks_or_default();
            total.delta += greeks.delta * quantity * CONTRACT_MULTIPLIER;
            total.gamma += greeks.gamma * quantity * CONTRACT_MULTIPLIER;
            total.vega += greeks.vega * quantity * CONTRACT_MULTIPLIER;
            total.theta += greeks.theta * quantity * CONTRACT_MULTIPLIER;
            total.rho += greeks.rho * quantity * CONTRACT_MULTIPLIER;
        }

        total.delta *= strategy_quantity;
        total.gamma *= strategy_quantity;
        total.vega *= strategy_quantity;
        total.theta *= strategy_quantity;
        total.rho *= strategy_quantity;

        Ok(total)
    }

    pub fn aggregate_model_greeks(
        positions: &[StrategyValuationPosition],
        underlying_price: f64,
        evaluation_time: &str,
        rate: f64,
        dividend_yield: Option<f64>,
        long_volatility_shift: Option<f64>,
        strategy_quantity: f64,
    ) -> OptionResult<Greeks> {
        let strategy_quantity = validate_strategy_quantity(strategy_quantity)?;
        ensure_positive(
            "invalid_strategy_payoff_input",
            "underlying_price",
            underlying_price,
        )?;
        ensure_finite("invalid_strategy_payoff_input", "rate", rate)?;
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

        let mut total = Greeks::default();
        for position in positions {
            validate_strategy_position(position)?;
            let quantity = f64::from(position.quantity);
            let years = valuation_years(&position.contract.expiration_date, evaluation_time)?;
            if years <= 0.0 {
                continue;
            }
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

            let greeks = pricing::greeks_black_scholes(&crate::BlackScholesInput {
                spot: underlying_price,
                strike: position.contract.strike,
                years,
                rate,
                dividend_yield,
                volatility: implied_volatility,
                option_right: position.contract.option_right.clone(),
            })?;

            total.delta += greeks.delta * quantity * CONTRACT_MULTIPLIER;
            total.gamma += greeks.gamma * quantity * CONTRACT_MULTIPLIER;
            total.vega += greeks.vega * quantity * CONTRACT_MULTIPLIER;
            total.theta += greeks.theta * quantity * CONTRACT_MULTIPLIER;
            total.rho += greeks.rho * quantity * CONTRACT_MULTIPLIER;
        }

        total.delta *= strategy_quantity;
        total.gamma *= strategy_quantity;
        total.vega *= strategy_quantity;
        total.theta *= strategy_quantity;
        total.rho *= strategy_quantity;

        Ok(total)
    }
}

pub fn strategy_pnl(input: &StrategyPnlInput) -> OptionResult<f64> {
    OptionStrategy::prepare_with_rate(
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
    OptionStrategy::prepare_with_rate(
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
