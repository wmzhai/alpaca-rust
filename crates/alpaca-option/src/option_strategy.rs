use crate::contract;
use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::pricing;
use crate::snapshot;
use crate::types::{
    Greeks, OptionContract, OptionPosition, OptionRight, OptionStrategyCurvePoint,
    OptionStrategyInput, StrategyBreakEvenInput, StrategyBreakEvenSideInput, StrategyPnlInput,
    StrategyPnlPeak, StrategyPnlPeakSearchInput, StrategyPositionTotals,
};
use crate::DEFAULT_RISK_FREE_RATE;
use alpaca_time::clock;
use alpaca_time::expiration;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

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
    positions: Vec<OptionPosition>,
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

fn strategy_position_contract(position: &OptionPosition) -> OptionResult<OptionContract> {
    contract::parse_occ_symbol(position.occ_symbol()).ok_or_else(|| {
        OptionError::new(
            "invalid_occ_symbol",
            format!("invalid occ symbol: {}", position.occ_symbol()),
        )
    })
}

fn validate_strategy_position(
    position: &OptionPosition,
    contract: &OptionContract,
) -> OptionResult<()> {
    ensure_positive(
        "invalid_strategy_payoff_input",
        "contract.strike",
        contract.strike,
    )?;
    if position.qty == 0 {
        return Err(OptionError::new(
            "invalid_strategy_payoff_input",
            "quantity must not be zero",
        ));
    }
    ensure_finite(
        "invalid_strategy_payoff_input",
        "avg_cost",
        position.avg_cost(),
    )?;
    if let Some(implied_volatility) = position
        .snapshot_ref()
        .and_then(|snapshot| snapshot.implied_volatility)
    {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "implied_volatility",
            implied_volatility,
        )?;
    }
    if let Some(snapshot) = position.snapshot_ref() {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "mark_price",
            snapshot.price(),
        )?;
        ensure_finite(
            "invalid_strategy_payoff_input",
            "reference_underlying_price",
            snapshot.underlying_price(),
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

fn strategy_entry_cost(positions: &[OptionPosition], entry_cost: Option<f64>) -> OptionResult<f64> {
    if let Some(entry_cost) = entry_cost {
        ensure_finite("invalid_strategy_payoff_input", "entry_cost", entry_cost)?;
        return Ok(entry_cost);
    }

    let mut total = 0.0;
    for position in positions {
        total += position.avg_cost() * f64::from(position.qty) * CONTRACT_MULTIPLIER;
    }
    Ok(total)
}

fn prepare_strategy_context(
    positions: &[OptionPosition],
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
        let contract = strategy_position_contract(position)?;
        validate_strategy_position(position, &contract)?;
        let quantity = f64::from(position.qty);
        let years = valuation_years(&contract.expiration_date, evaluation_time)?;
        let implied_volatility = if years <= 0.0 {
            None
        } else {
            let mut implied_volatility = position
                .snapshot_ref()
                .and_then(|snapshot| snapshot.implied_volatility)
                .ok_or_else(|| {
                    OptionError::new(
                        "invalid_strategy_payoff_input",
                        format!(
                            "implied_volatility is required before expiration: {}",
                            position.occ_symbol()
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
            option_right: contract.option_right,
            strike: contract.strike,
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

fn expiry_intrinsic_greeks(
    underlying_price: f64,
    strike: f64,
    option_right: &OptionRight,
) -> Greeks {
    let delta = match option_right {
        OptionRight::Call if underlying_price > strike => 1.0,
        OptionRight::Call if underlying_price < strike => 0.0,
        OptionRight::Call => 0.5,
        OptionRight::Put if underlying_price < strike => -1.0,
        OptionRight::Put if underlying_price > strike => 0.0,
        OptionRight::Put => -0.5,
    };

    Greeks {
        delta,
        ..Default::default()
    }
}

fn strategy_greeks_prepared(
    positions: &[PreparedStrategyLeg],
    underlying_price: f64,
    rate: f64,
    dividend_yield: f64,
) -> OptionResult<Greeks> {
    ensure_positive(
        "invalid_strategy_payoff_input",
        "underlying_price",
        underlying_price,
    )?;
    ensure_finite("invalid_strategy_payoff_input", "rate", rate)?;

    let mut total = Greeks::default();
    for position in positions {
        let greeks = if position.years <= 0.0 {
            expiry_intrinsic_greeks(underlying_price, position.strike, &position.option_right)
        } else {
            pricing::greeks_black_scholes(&crate::BlackScholesInput {
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

        total.delta += greeks.delta * position.quantity * CONTRACT_MULTIPLIER;
        total.gamma += greeks.gamma * position.quantity * CONTRACT_MULTIPLIER;
        total.vega += greeks.vega * position.quantity * CONTRACT_MULTIPLIER;
        total.theta += greeks.theta * position.quantity * CONTRACT_MULTIPLIER;
        total.rho += greeks.rho * position.quantity * CONTRACT_MULTIPLIER;
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

pub fn unique_break_even_points(points: impl IntoIterator<Item = f64>, tolerance: f64) -> Vec<f64> {
    let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        1e-6
    };
    let mut unique = Vec::new();
    for point in points {
        if !point.is_finite() {
            continue;
        }
        if unique
            .iter()
            .any(|existing: &f64| (*existing - point).abs() <= tolerance * 10.0)
        {
            continue;
        }
        unique.push(point);
    }
    unique.sort_by(|left, right| left.total_cmp(right));
    unique
}

pub fn strategy_position_totals(
    positions: &[OptionPosition],
    strategy_quantity: i32,
) -> StrategyPositionTotals {
    let quantity = Decimal::from(strategy_quantity);
    let quantity_abs = Decimal::from(strategy_quantity.unsigned_abs());
    let mut value = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    let mut spread = Decimal::ZERO;

    for position in positions {
        value += position.value();
        cost += position.cost();

        let spread_per_contract =
            alpaca_core::decimal::from_f64(snapshot::spread(&position.snapshot), 2);
        spread +=
            spread_per_contract * Decimal::from(position.qty.unsigned_abs()) * Decimal::from(100);
    }

    value *= quantity;
    cost *= quantity;
    spread *= quantity_abs;

    let cost_f64 = cost.to_f64().unwrap_or(0.0);
    let spread_rate = if cost_f64.abs() > 1e-10 {
        let spread_f64 = spread.to_f64().unwrap_or(0.0);
        Some(spread_f64 / cost_f64.abs())
    } else {
        None
    };

    StrategyPositionTotals {
        value,
        cost,
        spread,
        spread_rate,
    }
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
    pub fn expiration_time(positions: &[OptionPosition]) -> OptionResult<String> {
        let mut expiration_dates = Vec::with_capacity(positions.len());
        for position in positions {
            let contract = strategy_position_contract(position)?;
            if !contract.expiration_date.trim().is_empty() {
                expiration_dates.push(contract.expiration_date);
            }
        }
        let expiration_date = expiration_dates
            .iter()
            .map(String::as_str)
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
        positions: &[OptionPosition],
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
        positions: &[OptionPosition],
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
            positions: positions.to_vec(),
            prepared,
            entry_cost,
            rate,
            dividend_yield,
        })
    }

    pub fn prepare_mark_calibrated(
        positions: &[OptionPosition],
        evaluation_time: &str,
        entry_cost: Option<f64>,
        dividend_yield: Option<f64>,
    ) -> OptionResult<Self> {
        let dividend_yield = dividend_yield.unwrap_or(0.0);
        ensure_finite(
            "invalid_strategy_payoff_input",
            "dividend_yield",
            dividend_yield,
        )?;
        let calibrated: Vec<OptionPosition> = positions
            .iter()
            .map(|position| position.with_mark_calibrated_iv(evaluation_time, dividend_yield, None))
            .collect();
        Self::prepare(
            &calibrated,
            evaluation_time,
            entry_cost,
            Some(dividend_yield),
            None,
        )
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

    pub fn positions(&self) -> &[OptionPosition] {
        &self.positions
    }

    fn prepared_greeks_at(&self, underlying_price: f64) -> OptionResult<Greeks> {
        strategy_greeks_prepared(
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

    pub fn find_break_even_left(
        &self,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        self.find_break_even_toward(input.pivot, input.boundary, -input.scan_step.abs(), input)
    }

    pub fn find_break_even_right(
        &self,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        self.find_break_even_toward(input.pivot, input.boundary, input.scan_step.abs(), input)
    }

    fn find_break_even_toward(
        &self,
        pivot: f64,
        boundary: f64,
        signed_step: f64,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        let tolerance = input.tolerance.unwrap_or(1e-6);
        let max_iterations = input.max_iterations.unwrap_or(100);

        ensure_finite("invalid_strategy_payoff_input", "pivot", pivot)?;
        ensure_finite("invalid_strategy_payoff_input", "boundary", boundary)?;
        ensure_positive(
            "invalid_strategy_payoff_input",
            "scan_step",
            input.scan_step.abs(),
        )?;
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        if max_iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_iterations must be greater than zero",
            ));
        }
        if (signed_step < 0.0 && boundary >= pivot) || (signed_step > 0.0 && boundary <= pivot) {
            return Ok(None);
        }

        let mut previous = pivot;
        let mut previous_value = self.pnl_at(previous)?;
        if previous_value.abs() <= tolerance {
            return Ok(Some(previous));
        }

        loop {
            let next = if signed_step < 0.0 {
                (previous + signed_step).max(boundary)
            } else {
                (previous + signed_step).min(boundary)
            };
            let next_value = self.pnl_at(next)?;
            if next_value.abs() <= tolerance {
                return Ok(Some(next));
            }
            if next_value.signum() != previous_value.signum() {
                let low = previous.min(next);
                let high = previous.max(next);
                return self.break_even_between(low, high, Some(tolerance), Some(max_iterations));
            }
            if next == boundary {
                break;
            }
            previous = next;
            previous_value = next_value;
        }

        Ok(None)
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

    pub fn maximize_pnl_in_range(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        iterations: usize,
    ) -> OptionResult<StrategyPnlPeak> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }
        if iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "iterations must be greater than zero",
            ));
        }

        let mut left = lower_bound;
        let mut right = upper_bound;
        for _ in 0..iterations {
            let third = (right - left) / 3.0;
            let mid_left = left + third;
            let mid_right = right - third;
            let left_value = self.pnl_at(mid_left)?;
            let right_value = self.pnl_at(mid_right)?;

            if left_value < right_value {
                left = mid_left;
            } else {
                right = mid_right;
            }
        }

        let spot = (left + right) / 2.0;
        let pnl = self.pnl_at(spot)?;
        ensure_finite("invalid_strategy_payoff_input", "peak.pnl", pnl)?;
        Ok(StrategyPnlPeak { spot, pnl })
    }

    pub fn pnl_peak_from_current(
        &self,
        input: &StrategyPnlPeakSearchInput,
    ) -> OptionResult<Option<StrategyPnlPeak>> {
        ensure_positive(
            "invalid_strategy_payoff_input",
            "current_price",
            input.current_price,
        )?;
        ensure_positive(
            "invalid_strategy_payoff_input",
            "left_boundary",
            input.left_boundary,
        )?;
        if input.left_boundary >= input.current_price {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "left_boundary must be less than current_price: {} >= {}",
                    input.left_boundary, input.current_price
                ),
            ));
        }
        ensure_finite(
            "invalid_strategy_payoff_input",
            "right_boundary",
            input.right_boundary,
        )?;
        if input.right_boundary <= input.current_price {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "right_boundary must be greater than current_price: {} <= {}",
                    input.right_boundary, input.current_price
                ),
            ));
        }
        if input.right_boundary <= input.left_boundary {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "right_boundary must be greater than left_boundary: {} <= {}",
                    input.right_boundary, input.left_boundary
                ),
            ));
        }
        let tolerance = input.tolerance.unwrap_or(1e-6);
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        let max_search_steps = input.max_search_steps.unwrap_or(512);
        if max_search_steps == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_search_steps must be greater than zero",
            ));
        }

        let fallback_step = (input.current_price * 0.005).clamp(0.25, 5.0);
        let step = input
            .step_hint
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(fallback_step)
            .clamp(0.05, input.current_price.max(1.0) * 0.05);
        let current_pnl = self.pnl_at(input.current_price)?;
        let left_spot = (input.current_price - step).max(input.left_boundary);
        let right_spot = (input.current_price + step).min(input.right_boundary);
        let left_pnl = if left_spot < input.current_price {
            self.pnl_at(left_spot)?
        } else {
            f64::NEG_INFINITY
        };
        let right_pnl = if right_spot > input.current_price {
            self.pnl_at(right_spot)?
        } else {
            f64::NEG_INFINITY
        };

        let peak = if left_pnl <= current_pnl + tolerance && right_pnl <= current_pnl + tolerance {
            self.maximize_pnl_in_range(left_spot, right_spot, 80)?
        } else {
            let direction = if right_pnl >= left_pnl { 1.0 } else { -1.0 };
            let mut previous_spot = input.current_price;
            let mut best_spot = if direction > 0.0 {
                right_spot
            } else {
                left_spot
            };
            let mut best_pnl = if direction > 0.0 { right_pnl } else { left_pnl };
            let mut refined = None;

            for _ in 0..max_search_steps {
                let next_spot =
                    (best_spot + direction * step).clamp(input.left_boundary, input.right_boundary);
                if (next_spot - best_spot).abs() <= f64::EPSILON {
                    break;
                }
                let next_pnl = self.pnl_at(next_spot)?;
                if next_pnl > best_pnl + tolerance {
                    previous_spot = best_spot;
                    best_spot = next_spot;
                    best_pnl = next_pnl;
                    continue;
                }

                let lower = previous_spot.min(next_spot);
                let upper = previous_spot.max(next_spot);
                refined = Some(self.maximize_pnl_in_range(lower, upper, 80)?);
                break;
            }

            refined.unwrap_or(StrategyPnlPeak {
                spot: best_spot,
                pnl: best_pnl,
            })
        };

        if peak.pnl <= tolerance {
            return Ok(None);
        }
        Ok(Some(peak))
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

    pub fn greeks_at(&self, underlying_price: f64, strategy_quantity: f64) -> OptionResult<Greeks> {
        let strategy_quantity = validate_strategy_quantity(strategy_quantity)?;
        let mut total = self.prepared_greeks_at(underlying_price)?;

        total.delta *= strategy_quantity;
        total.gamma *= strategy_quantity;
        total.vega *= strategy_quantity;
        total.theta *= strategy_quantity;
        total.rho *= strategy_quantity;

        Ok(total)
    }

    pub fn aggregate_model_greeks(
        positions: &[OptionPosition],
        underlying_price: f64,
        evaluation_time: &str,
        dividend_yield: Option<f64>,
        long_volatility_shift: Option<f64>,
        strategy_quantity: f64,
    ) -> OptionResult<Greeks> {
        Self::prepare(
            positions,
            evaluation_time,
            Some(0.0),
            dividend_yield,
            long_volatility_shift,
        )?
        .greeks_at(underlying_price, strategy_quantity)
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
