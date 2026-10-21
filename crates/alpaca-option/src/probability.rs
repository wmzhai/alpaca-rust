use crate::error::{OptionError, OptionResult};
use crate::numeric::normal_cdf;
use crate::rate::risk_free_rate_for_years;

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_probability_input",
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn ensure_positive(name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_probability_input",
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

pub fn expiry_probability_in_range(
    spot: f64,
    lower_price: f64,
    upper_price: f64,
    years: f64,
    dividend_yield: f64,
    volatility: f64,
) -> OptionResult<f64> {
    expiry_probability_in_range_with_rate(
        spot,
        lower_price,
        upper_price,
        years,
        risk_free_rate_for_years(years),
        dividend_yield,
        volatility,
    )
}

pub fn expiry_probability_in_range_with_rate(
    spot: f64,
    lower_price: f64,
    upper_price: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("lower_price", lower_price)?;
    ensure_positive("upper_price", upper_price)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_finite("dividend_yield", dividend_yield)?;
    ensure_positive("volatility", volatility)?;
    if lower_price >= upper_price {
        return Err(OptionError::new(
            "invalid_probability_input",
            format!("lower_price must be less than upper_price: {lower_price} >= {upper_price}"),
        ));
    }

    let sigma_sqrt_t = volatility * years.sqrt();
    let z_lower = (spot / lower_price).ln() / sigma_sqrt_t;
    let z_upper = (spot / upper_price).ln() / sigma_sqrt_t;

    Ok(normal_cdf(z_lower) - normal_cdf(z_upper))
}

/// One breakeven and the direction PnL takes as the underlying price rises.
///
/// `direction` is `1` when PnL changes from loss to profit, and `-1` when it
/// changes from profit to loss. Any number of points can be combined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectedBreakEven {
    pub at: f64,
    pub direction: i8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChanceTarget {
    pub above: f64,
    pub below: f64,
}

/// Zero-drift chance the terminal price is above or below `target`.
///
/// `days` is the calendar span used by the lognormal, divided by 365.
/// Rate and dividend do not enter. At or before expiry the chance is 0 or 1,
/// except exactly at the spot, which stays one half.
pub fn chance_of_target(
    spot: f64,
    target: f64,
    days: f64,
    volatility: f64,
) -> OptionResult<ChanceTarget> {
    ensure_positive("spot", spot)?;
    ensure_finite("target", target)?;
    ensure_finite("days", days)?;
    ensure_finite("volatility", volatility)?;
    if target < 0.0 {
        return Ok(ChanceTarget {
            above: 1.0,
            below: 0.0,
        });
    }
    if days <= 0.0 {
        if spot > target {
            return Ok(ChanceTarget {
                above: 1.0,
                below: 0.0,
            });
        }
        if target > spot {
            return Ok(ChanceTarget {
                above: 0.0,
                below: 1.0,
            });
        }
        return Ok(ChanceTarget {
            above: 0.5,
            below: 0.5,
        });
    }
    ensure_positive("volatility", volatility)?;
    let denominator = volatility * (days / 365.0).sqrt();
    let below = normal_cdf((target / spot).ln() / denominator);
    Ok(ChanceTarget {
        above: 1.0 - below,
        below,
    })
}

pub struct ChanceOfProfitInput<'a> {
    pub spot: f64,
    pub days: f64,
    pub volatility: f64,
    pub breakevens: &'a [DirectedBreakEven],
    /// Best PnL. `+∞` is allowed. A non-positive value means profit is impossible.
    pub max_profit: f64,
    /// Worst PnL. `-∞` is allowed. A non-negative value means a loss is impossible.
    pub min_pnl: f64,
    /// PnL used only when `breakevens` is empty.
    pub pnl_without_break_even: f64,
}

/// Probability of profit from every breakeven direction.
///
/// The mass below each breakeven is added or removed according to that point's
/// direction. Two profit islands, or any longer sequence, use the same sum.
/// A non-positive best PnL is 0. A worst PnL that is still profit is 1.
pub fn chance_of_profit(input: ChanceOfProfitInput<'_>) -> OptionResult<f64> {
    if !input.max_profit.is_finite() && !input.max_profit.is_infinite() {
        return Err(OptionError::new(
            "invalid_probability_input",
            format!("max_profit must be finite or infinite: {}", input.max_profit),
        ));
    }
    if !input.min_pnl.is_finite() && !input.min_pnl.is_infinite() {
        return Err(OptionError::new(
            "invalid_probability_input",
            format!("min_pnl must be finite or infinite: {}", input.min_pnl),
        ));
    }
    ensure_finite("pnl_without_break_even", input.pnl_without_break_even)?;
    if input.max_profit <= 0.0 {
        return Ok(0.0);
    }
    if input.min_pnl >= 0.0 {
        return Ok(1.0);
    }
    if input.breakevens.is_empty() {
        return Ok(if input.pnl_without_break_even > 0.0 {
            1.0
        } else {
            0.0
        });
    }

    let mut ordered = input.breakevens.to_vec();
    ordered.sort_by(|left, right| {
        left.at
            .partial_cmp(&right.at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut unique = Vec::with_capacity(ordered.len());
    for item in ordered {
        ensure_finite("breakeven", item.at)?;
        if item.direction != 1 && item.direction != -1 {
            return Err(OptionError::new(
                "invalid_probability_input",
                format!("direction must be 1 or -1: {}", item.direction),
            ));
        }
        if unique
            .last()
            .is_some_and(|previous: &DirectedBreakEven| (previous.at - item.at).abs() <= 1e-8)
        {
            continue;
        }
        unique.push(item);
    }
    let last_direction = unique.last().map(|item| item.direction).unwrap_or(0);
    let mut share = if last_direction > 0 { 1.0 } else { 0.0 };
    for item in &unique {
        let below = chance_of_target(input.spot, item.at, input.days, input.volatility)?.below;
        share -= below * f64::from(item.direction);
    }
    Ok(share)
}

fn pnl_sign(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn push_directed(roots: &mut Vec<DirectedBreakEven>, at: f64, direction: i8) {
    if !at.is_finite() || (direction != 1 && direction != -1) {
        return;
    }
    if roots
        .last()
        .is_some_and(|previous| (previous.at - at).abs() <= 1e-8)
    {
        return;
    }
    roots.push(DirectedBreakEven { at, direction });
}

/// Linear roots between samples, with a direction at each sign change.
pub fn directed_break_evens(prices: &[f64], pnls: &[f64]) -> OptionResult<Vec<DirectedBreakEven>> {
    if prices.len() != pnls.len() {
        return Err(OptionError::new(
            "invalid_probability_input",
            format!(
                "prices and pnls must have the same length: {} != {}",
                prices.len(),
                pnls.len()
            ),
        ));
    }
    let mut roots = Vec::new();
    for index in 1..prices.len() {
        let left_price = prices[index - 1];
        let right_price = prices[index];
        let left = pnls[index - 1];
        let right = pnls[index];
        if !left_price.is_finite()
            || !right_price.is_finite()
            || !left.is_finite()
            || !right.is_finite()
            || right_price <= left_price
        {
            continue;
        }
        let left_sign = pnl_sign(left);
        let right_sign = pnl_sign(right);
        if left_sign == right_sign {
            continue;
        }
        let span = right - left;
        let at = if span == 0.0 {
            left_price
        } else {
            left_price - left * (right_price - left_price) / span
        };
        let direction = if left_sign < right_sign { 1 } else { -1 };
        push_directed(&mut roots, at, direction);
    }
    Ok(roots)
}

/// Same sign scan, then bisection of `evaluate` down to `tolerance`.
pub fn polished_directed_break_evens<F>(
    prices: &[f64],
    mut evaluate: F,
    tolerance: f64,
    max_iterations: usize,
) -> OptionResult<Vec<DirectedBreakEven>>
where
    F: FnMut(f64) -> f64,
{
    ensure_positive("tolerance", tolerance)?;
    if max_iterations == 0 {
        return Err(OptionError::new(
            "invalid_probability_input",
            "max_iterations must be positive",
        ));
    }
    let mut sampled = Vec::with_capacity(prices.len());
    for price in prices {
        ensure_finite("price", *price)?;
        let pnl = evaluate(*price);
        if pnl.is_finite() {
            sampled.push((*price, pnl));
        }
    }
    let mut roots = Vec::new();
    for index in 1..sampled.len() {
        let (left_price, left) = sampled[index - 1];
        let (right_price, right) = sampled[index];
        if right_price <= left_price {
            continue;
        }
        let left_sign = pnl_sign(left);
        let right_sign = pnl_sign(right);
        if left_sign == right_sign {
            continue;
        }
        let direction = if left_sign < right_sign { 1 } else { -1 };
        let at = bisect_root(&mut evaluate, left_price, right_price, tolerance, max_iterations);
        push_directed(&mut roots, at, direction);
    }
    Ok(roots)
}

fn slope_of<F>(evaluate: &mut F, price: f64, bump: f64) -> f64
where
    F: FnMut(f64) -> f64,
{
    (evaluate(price + bump) - evaluate(price)) / bump
}

fn bisect_slope<F>(evaluate: &mut F, mut left: f64, mut right: f64, bump: f64, max_iterations: usize) -> f64
where
    F: FnMut(f64) -> f64,
{
    let mut left_value = slope_of(evaluate, left, bump);
    for _ in 0..max_iterations {
        if (right - left).abs() <= bump {
            break;
        }
        let mid = (left + right) / 2.0;
        let mid_value = slope_of(evaluate, mid, bump);
        if !mid_value.is_finite() || mid_value == 0.0 {
            return mid;
        }
        if left_value * mid_value <= 0.0 {
            right = mid;
        } else {
            left = mid;
            left_value = mid_value;
        }
    }
    (left + right) / 2.0
}

fn bisect_root<F>(
    evaluate: &mut F,
    mut left: f64,
    mut right: f64,
    tolerance: f64,
    max_iterations: usize,
) -> f64
where
    F: FnMut(f64) -> f64,
{
    let mut left_value = evaluate(left);
    let right_value = evaluate(right);
    if !left_value.is_finite() || !right_value.is_finite() || left_value * right_value > 0.0 {
        let span = right_value - left_value;
        if span == 0.0 {
            return (left + right) / 2.0;
        }
        return left - left_value * (right - left) / span;
    }
    for _ in 0..max_iterations {
        if (right - left).abs() <= tolerance {
            break;
        }
        let mid = (left + right) / 2.0;
        let mid_value = evaluate(mid);
        if !mid_value.is_finite() || mid_value == 0.0 {
            return mid;
        }
        if left_value * mid_value <= 0.0 {
            right = mid;
        } else {
            left = mid;
            left_value = mid_value;
        }
    }
    (left + right) / 2.0
}

/// Value approached from one side. A still-diverging wing is infinite.
pub fn approaching_value<F>(mut evaluate: F, target: f64, direction: f64) -> f64
where
    F: FnMut(f64) -> f64,
{
    let bound = if target.is_finite() {
        target
    } else if target > 0.0 {
        f64::MAX
    } else {
        f64::MIN
    };
    let step = 1.0_f64.max(bound.abs() / 100.0);
    let mut previous_delta: Option<f64> = None;
    let mut previous: Option<f64> = None;
    let mut settled = false;
    for index in 0..10 {
        let price = bound - ((10 - index) as f64 / 10.0) * step * direction;
        let value = evaluate(price);
        if let Some(prior) = previous {
            let delta = (value - prior).abs();
            if (index > 1 && previous_delta.is_some_and(|earlier| delta + 1e-4 < earlier)) || delta <= 1e-4
            {
                settled = true;
            }
            previous_delta = Some(delta);
        }
        previous = Some(value);
    }
    let at_bound = evaluate(bound);
    if !settled {
        if let Some(prior) = previous {
            let sign = (at_bound - prior).signum();
            if sign != 0.0 {
                return sign * f64::INFINITY;
            }
        }
    }
    at_bound
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfitLossBounds {
    pub max_profit: f64,
    pub min_pnl: f64,
}

/// Five implied moves around the spot, floored at 0, then expanded to every positive strike.
fn profit_loss_scan_window(spot: f64, implied_move: f64, strikes: &[f64]) -> (f64, f64) {
    let move_size = if implied_move.is_finite() {
        implied_move.max(0.0)
    } else {
        0.0
    };
    let pad = 5.0 * move_size;
    let spot = if spot.is_finite() { spot } else { 0.0 };
    let mut lower = (spot - pad).max(0.0);
    let mut upper = spot + pad;
    for strike in strikes {
        if *strike > 0.0 {
            if *strike < lower {
                lower = *strike;
            }
            if *strike > upper {
                upper = *strike;
            }
        }
    }
    if upper <= lower {
        upper = lower + 1.0;
    }
    (lower, upper)
}

/// Best and worst PnL. One expiration uses strike knots. Several use derivative zeros and price 0.
pub fn profit_loss_bounds<F>(
    mut evaluate: F,
    spot: f64,
    strikes: &[f64],
    single_expiration: bool,
    implied_move: f64,
    samples: usize,
) -> OptionResult<ProfitLossBounds>
where
    F: FnMut(f64) -> f64,
{
    ensure_finite("spot", spot)?;
    let mut listed: Vec<f64> = strikes.iter().copied().filter(|strike| *strike > 0.0).collect();
    listed.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    if single_expiration {
        let right = spot.max(0.0) * 100.0;
        let mut knots = vec![0.0];
        if listed.is_empty() {
            knots.push(spot + right);
        } else {
            knots.extend_from_slice(&listed);
            knots.push(listed[listed.len() - 1] + right);
        }
        let mut max_profit = f64::NEG_INFINITY;
        let mut min_pnl = f64::INFINITY;
        let mut previous: Option<f64> = None;
        for (index, price) in knots.iter().copied().enumerate() {
            let value = evaluate(price);
            max_profit = max_profit.max(value);
            min_pnl = min_pnl.min(value);
            if index + 1 == knots.len() {
                if let Some(prior) = previous {
                    if value > prior + 1e-4 {
                        max_profit = f64::INFINITY;
                    }
                    if value < prior - 1e-4 {
                        min_pnl = f64::NEG_INFINITY;
                    }
                }
            }
            previous = Some(value);
        }
        return Ok(ProfitLossBounds { max_profit, min_pnl });
    }

    let (lower, upper) = profit_loss_scan_window(spot, implied_move, &listed);
    let count = samples.max(1);
    let step = (upper - lower) / count as f64;
    let bump = 0.001;
    let mut zeros = Vec::new();
    let mut previous_slope: Option<f64> = None;
    for index in 0..count {
        let price = lower + index as f64 * step;
        if price <= 0.0 {
            continue;
        }
        let next = slope_of(&mut evaluate, price, bump);
        if next.is_nan() {
            previous_slope = None;
            continue;
        }
        let left = price - step;
        if let Some(prior) = previous_slope {
            if left > 0.0
                && prior.signum() != 0.0
                && next.signum() != 0.0
                && prior.signum() != next.signum()
            {
                zeros.push(bisect_slope(&mut evaluate, left, price, bump, 30));
            }
        }
        previous_slope = Some(next);
    }
    let mut max_profit = f64::NEG_INFINITY;
    let mut min_pnl = f64::INFINITY;
    let mut consider = |value: f64| {
        if value.is_nan() {
            return;
        }
        max_profit = max_profit.max(value);
        min_pnl = min_pnl.min(value);
    };
    for zero in zeros {
        if zero > 0.0 {
            consider(evaluate(zero));
        }
    }
    consider(evaluate(0.0));
    consider(approaching_value(&mut evaluate, spot.max(0.0) * 100.0, 1.0));
    Ok(ProfitLossBounds { max_profit, min_pnl })
}
