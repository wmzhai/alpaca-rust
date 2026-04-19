use crate::error::{OptionError, OptionResult};
use crate::math::round_to_fixture_years;
use crate::numeric::{normal_cdf, normal_pdf};
use crate::types::OptionRight;
use serde::{Deserialize, Serialize};

const DISCRETE_DIVIDEND_TIME_STEPS_PER_YEAR: f64 = 300.0;
const DISCRETE_DIVIDEND_SPACE_STEPS: usize = 300;

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_math_input",
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
            "invalid_math_input",
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

fn parse_option_right(option_right: &str) -> OptionResult<OptionRight> {
    OptionRight::from_str(option_right).map_err(|_| {
        OptionError::new(
            "invalid_math_input",
            format!("invalid option right: {option_right}"),
        )
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashDividend {
    pub time: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CashDividendModel {
    Spot,
    Escrowed,
}

impl CashDividendModel {
    fn parse(input: &str) -> OptionResult<Self> {
        match input {
            "spot" => Ok(Self::Spot),
            "escrowed" => Ok(Self::Escrowed),
            _ => Err(OptionError::new(
                "invalid_math_input",
                format!("invalid cash_dividend_model: {input}"),
            )),
        }
    }
}

fn validate_inputs(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<OptionRight> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_finite("dividend_yield", dividend_yield)?;
    ensure_positive("volatility", volatility)?;
    parse_option_right(option_right)
}

fn carry(rate: f64, dividend_yield: f64) -> f64 {
    rate - dividend_yield
}

fn discount(rate: f64, years: f64) -> f64 {
    (-rate * years).exp()
}

fn intrinsic(option_right: &OptionRight, spot: f64, strike: f64) -> f64 {
    match option_right {
        OptionRight::Call => (spot - strike).max(0.0),
        OptionRight::Put => (strike - spot).max(0.0),
    }
}

fn gbs_d1_d2(spot: f64, strike: f64, years: f64, carry: f64, volatility: f64) -> (f64, f64) {
    let sqrt_years = years.sqrt();
    let sigma_sqrt_t = volatility * sqrt_years;
    let d1 = ((spot / strike).ln() + (carry + 0.5 * volatility * volatility) * years)
        / sigma_sqrt_t;
    (d1, d1 - sigma_sqrt_t)
}

fn gbs_price(
    option_right: &OptionRight,
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    carry: f64,
    volatility: f64,
) -> f64 {
    let dividend_discount = ((carry - rate) * years).exp();
    let risk_free_discount = discount(rate, years);
    if volatility == 0.0 {
        let forward = spot * (carry * years).exp();
        return risk_free_discount
            * match option_right {
                OptionRight::Call => (forward - strike).max(0.0),
                OptionRight::Put => (strike - forward).max(0.0),
            };
    }

    let sign = match option_right {
        OptionRight::Call => 1.0,
        OptionRight::Put => -1.0,
    };
    let (d1, d2) = gbs_d1_d2(spot, strike, years, carry, volatility);
    sign * (spot * dividend_discount * normal_cdf(sign * d1)
        - strike * risk_free_discount * normal_cdf(sign * d2))
}

fn critical_price(
    option_right: &OptionRight,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    tolerance: f64,
) -> OptionResult<f64> {
    let variance = volatility * volatility * years;
    let risk_free_discount = discount(rate, years);
    if risk_free_discount > 1.0 + 1e-12 {
        return Err(OptionError::new(
            "unsupported_math_input",
            "american approximation does not support negative rates",
        ));
    }
    let dividend_discount = (-dividend_yield * years).exp();
    let sqrt_variance = variance.sqrt();
    let n = 2.0 * (dividend_discount / risk_free_discount).ln() / variance;
    let m = -2.0 * risk_free_discount.ln() / variance;
    let carry_term = (dividend_discount / risk_free_discount).ln();

    let (qu, su, h, mut si) = match option_right {
        OptionRight::Call => {
            let qu = (-(n - 1.0) + ((n - 1.0) * (n - 1.0) + 4.0 * m).sqrt()) / 2.0;
            let su = strike / (1.0 - 1.0 / qu);
            let h = -(carry_term + 2.0 * sqrt_variance) * strike / (su - strike);
            let si = strike + (su - strike) * (1.0 - h.exp());
            (qu, su, h, si)
        }
        OptionRight::Put => {
            let qu = (-(n - 1.0) - ((n - 1.0) * (n - 1.0) + 4.0 * m).sqrt()) / 2.0;
            let su = strike / (1.0 - 1.0 / qu);
            let h = (carry_term - 2.0 * sqrt_variance) * strike / (strike - su);
            let si = su + (strike - su) * h.exp();
            (qu, su, h, si)
        }
    };
    let _ = (qu, su, h);

    let kappa = if (risk_free_discount - 1.0).abs() > 1e-12 {
        -2.0 * risk_free_discount.ln() / (variance * (1.0 - risk_free_discount))
    } else {
        2.0 / variance
    };

    let mut forward_si = si * dividend_discount / risk_free_discount;
    let mut d1 = (forward_si / strike).ln() / sqrt_variance + 0.5 * sqrt_variance;
    let mut temp = gbs_price(
        option_right,
        si,
        strike,
        years,
        rate,
        carry(rate, dividend_yield),
        volatility,
    );

    match option_right {
        OptionRight::Call => {
            let q = (-(n - 1.0) + ((n - 1.0) * (n - 1.0) + 4.0 * kappa).sqrt()) / 2.0;
            let mut lhs = si - strike;
            let mut rhs = temp + (1.0 - dividend_discount * normal_cdf(d1)) * si / q;
            let mut bi = dividend_discount * normal_cdf(d1) * (1.0 - 1.0 / q)
                + (1.0 - dividend_discount * normal_pdf(d1) / sqrt_variance) / q;

            while (lhs - rhs).abs() / strike > tolerance {
                si = (strike + rhs - bi * si) / (1.0 - bi);
                forward_si = si * dividend_discount / risk_free_discount;
                d1 = (forward_si / strike).ln() / sqrt_variance + 0.5 * sqrt_variance;
                lhs = si - strike;
                temp = gbs_price(
                    option_right,
                    si,
                    strike,
                    years,
                    rate,
                    carry(rate, dividend_yield),
                    volatility,
                );
                rhs = temp + (1.0 - dividend_discount * normal_cdf(d1)) * si / q;
                bi = dividend_discount * normal_cdf(d1) * (1.0 - 1.0 / q)
                    + (1.0 - dividend_discount * normal_pdf(d1) / sqrt_variance) / q;
            }
        }
        OptionRight::Put => {
            let q = (-(n - 1.0) - ((n - 1.0) * (n - 1.0) + 4.0 * kappa).sqrt()) / 2.0;
            let mut lhs = strike - si;
            let mut rhs = temp - (1.0 - dividend_discount * normal_cdf(-d1)) * si / q;
            let mut bi = -dividend_discount * normal_cdf(-d1) * (1.0 - 1.0 / q)
                - (1.0 + dividend_discount * normal_pdf(d1) / sqrt_variance) / q;

            while (lhs - rhs).abs() / strike > tolerance {
                si = (strike - rhs + bi * si) / (1.0 + bi);
                forward_si = si * dividend_discount / risk_free_discount;
                d1 = (forward_si / strike).ln() / sqrt_variance + 0.5 * sqrt_variance;
                lhs = strike - si;
                temp = gbs_price(
                    option_right,
                    si,
                    strike,
                    years,
                    rate,
                    carry(rate, dividend_yield),
                    volatility,
                );
                rhs = temp - (1.0 - dividend_discount * normal_cdf(-d1)) * si / q;
                bi = -dividend_discount * normal_cdf(-d1) * (1.0 - 1.0 / q)
                    - (1.0 + dividend_discount * normal_pdf(d1) / sqrt_variance) / q;
            }
        }
    }

    Ok(si)
}

fn bs1993_phi(
    spot: f64,
    years: f64,
    gamma: f64,
    h: f64,
    i: f64,
    rate: f64,
    carry: f64,
    volatility: f64,
) -> f64 {
    let sqrt_years = years.sqrt();
    let sigma_sq = volatility * volatility;
    let lambda_term = (-rate + gamma * carry + 0.5 * gamma * (gamma - 1.0) * sigma_sq) * years;
    let d = -((spot / h).ln() + (carry + (gamma - 0.5) * sigma_sq) * years)
        / (volatility * sqrt_years);
    let kappa = 2.0 * carry / sigma_sq + 2.0 * gamma - 1.0;
    lambda_term.exp()
        * spot.powf(gamma)
        * (normal_cdf(d) - (i / spot).powf(kappa) * normal_cdf(d - 2.0 * (i / spot).ln() / (volatility * sqrt_years)))
}

fn bs1993_call_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    carry: f64,
    volatility: f64,
) -> f64 {
    if carry >= rate {
        return gbs_price(
            &OptionRight::Call,
            spot,
            strike,
            years,
            rate,
            carry,
            volatility,
        );
    }

    let sigma_sq = volatility * volatility;
    let beta = (0.5 - carry / sigma_sq)
        + ((carry / sigma_sq - 0.5) * (carry / sigma_sq - 0.5) + 2.0 * rate / sigma_sq).sqrt();
    let b_infinity = beta / (beta - 1.0) * strike;
    let b0 = strike.max(rate / (rate - carry) * strike);
    let ht = -(carry * years + 2.0 * volatility * years.sqrt()) * b0 / (b_infinity - b0);
    let i = b0 + (b_infinity - b0) * (1.0 - ht.exp());
    let alpha = (i - strike) * i.powf(-beta);
    if spot >= i {
        return spot - strike;
    }

    alpha * spot.powf(beta)
        - alpha * bs1993_phi(spot, years, beta, i, i, rate, carry, volatility)
        + bs1993_phi(spot, years, 1.0, i, i, rate, carry, volatility)
        - bs1993_phi(spot, years, 1.0, strike, i, rate, carry, volatility)
        - strike * bs1993_phi(spot, years, 0.0, i, i, rate, carry, volatility)
        + strike * bs1993_phi(spot, years, 0.0, strike, i, rate, carry, volatility)
}

fn ju_quadratic_price_inner(
    option_right: &OptionRight,
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> OptionResult<f64> {
    let dividend_discount = (-dividend_yield * years).exp();
    if dividend_discount >= 1.0 && matches!(option_right, OptionRight::Call) {
        return Ok(gbs_price(
            option_right,
            spot,
            strike,
            years,
            rate,
            carry(rate, dividend_yield),
            volatility,
        ));
    }

    let variance = volatility * volatility * years;
    let sqrt_variance = variance.sqrt();
    let risk_free_discount = discount(rate, years);
    if risk_free_discount > 1.0 + 1e-12 {
        return Err(OptionError::new(
            "unsupported_math_input",
            "ju quadratic approximation does not support negative rates",
        ));
    }

    let european_price = gbs_price(
        option_right,
        spot,
        strike,
        years,
        rate,
        carry(rate, dividend_yield),
        volatility,
    );
    let sk = critical_price(
        option_right,
        strike,
        years,
        rate,
        dividend_yield,
        volatility,
        1e-6,
    )?;
    let forward_sk = sk * dividend_discount / risk_free_discount;
    let alpha = -2.0 * risk_free_discount.ln() / variance;
    let beta = 2.0 * (dividend_discount / risk_free_discount).ln() / variance;
    let h = 1.0 - risk_free_discount;
    if h.abs() < 1e-12 || risk_free_discount.ln().abs() < 1e-12 {
        return barone_adesi_whaley_price(
            spot,
            strike,
            years,
            rate,
            dividend_yield,
            volatility,
            option_right.as_str(),
        );
    }

    let phi = match option_right {
        OptionRight::Call => 1.0,
        OptionRight::Put => -1.0,
    };
    let temp_root = ((beta - 1.0) * (beta - 1.0) + (4.0 * alpha) / h).sqrt();
    let lambda = (-(beta - 1.0) + phi * temp_root) / 2.0;
    let lambda_prime = -phi * alpha / (h * h * temp_root);
    let black_sk = gbs_price(
        option_right,
        sk,
        strike,
        years,
        rate,
        carry(rate, dividend_yield),
        volatility,
    );
    let h_a = phi * (sk - strike) - black_sk;
    let d1_sk = ((forward_sk / strike).ln() + 0.5 * variance) / sqrt_variance;
    let d2_sk = d1_sk - sqrt_variance;
    let v_e_h = forward_sk * normal_pdf(d1_sk) / (alpha * sqrt_variance)
        - phi * forward_sk * normal_cdf(phi * d1_sk) * dividend_discount.ln() / risk_free_discount.ln()
        + phi * strike * normal_cdf(phi * d2_sk);
    let denominator = 2.0 * lambda + beta - 1.0;
    let b = (1.0 - h) * alpha * lambda_prime / (2.0 * denominator);
    let c = -((1.0 - h) * alpha / denominator)
        * (v_e_h / h_a + 1.0 / h + lambda_prime / denominator);
    let spot_ratio = (spot / sk).ln();
    let chi = spot_ratio * (b * spot_ratio + c);

    if phi * (sk - spot) > 0.0 {
        Ok(european_price + h_a * (spot / sk).powf(lambda) / (1.0 - chi))
    } else {
        Ok(phi * (spot - strike))
    }
}

pub fn tree_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
    steps: Option<usize>,
    use_richardson: Option<bool>,
) -> OptionResult<f64> {
    let option_right =
        validate_inputs(spot, strike, years, rate, dividend_yield, volatility, option_right)?;

    if matches!(option_right, OptionRight::Call) && dividend_yield <= 0.0 {
        return Ok(gbs_price(
            &option_right,
            spot,
            strike,
            years,
            rate,
            carry(rate, dividend_yield),
            volatility,
        ));
    }

    let tree_once = |steps: usize| -> OptionResult<f64> {
        if steps < 2 {
            return Err(OptionError::new(
                "invalid_math_input",
                "steps must be at least 2",
            ));
        }
        let dt = years / steps as f64;
        let growth = ((rate - dividend_yield) * dt).exp();
        let up = (volatility * dt.sqrt()).exp();
        let down = 1.0 / up;
        let probability = (growth - down) / (up - down);
        if !(0.0..1.0).contains(&probability) {
            return Err(OptionError::new(
                "invalid_math_input",
                "tree probability is out of bounds",
            ));
        }

        let discount_step = (-rate * dt).exp();
        let mut values = (0..=steps)
            .map(|index| {
                let stock = spot * down.powf((steps - index) as f64) * up.powf(index as f64);
                intrinsic(&option_right, stock, strike)
            })
            .collect::<Vec<_>>();

        for level in (1..=steps).rev() {
            for index in 0..level {
                let continuation =
                    discount_step * (probability * values[index + 1] + (1.0 - probability) * values[index]);
                let stock = spot * down.powf((level - 1 - index) as f64) * up.powf(index as f64);
                let exercise = intrinsic(&option_right, stock, strike);
                values[index] = continuation.max(exercise);
            }
        }

        Ok(values[0])
    };

    let mut base_steps = steps.unwrap_or(4000).max(200);
    if base_steps % 2 == 1 {
        base_steps += 1;
    }

    let coarse = tree_once(base_steps)?;
    if !use_richardson.unwrap_or(true) {
        return Ok(coarse);
    }
    let fine = tree_once(base_steps * 2)?;
    Ok(2.0 * fine - coarse)
}

pub fn barone_adesi_whaley_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<f64> {
    let option_right =
        validate_inputs(spot, strike, years, rate, dividend_yield, volatility, option_right)?;
    let carry = carry(rate, dividend_yield);
    if matches!(option_right, OptionRight::Call) && carry >= rate {
        return Ok(gbs_price(
            &option_right,
            spot,
            strike,
            years,
            rate,
            carry,
            volatility,
        ));
    }

    let variance = volatility * volatility * years;
    let sqrt_variance = variance.sqrt();
    let risk_free_discount = discount(rate, years);
    if risk_free_discount > 1.0 + 1e-12 {
        return Err(OptionError::new(
            "unsupported_math_input",
            "barone-adesi-whaley does not support negative rates",
        ));
    }
    let dividend_discount = (-dividend_yield * years).exp();
    let sk = critical_price(
        &option_right,
        strike,
        years,
        rate,
        dividend_yield,
        volatility,
        1e-6,
    )?;
    let forward_sk = sk * dividend_discount / risk_free_discount;
    let d1 = (forward_sk / strike).ln() / sqrt_variance + 0.5 * sqrt_variance;
    let n = 2.0 * (dividend_discount / risk_free_discount).ln() / variance;
    let kappa = if (risk_free_discount - 1.0).abs() > 1e-12 {
        -2.0 * risk_free_discount.ln() / (variance * (1.0 - risk_free_discount))
    } else {
        2.0 / variance
    };

    match option_right {
        OptionRight::Call => {
            let q = (-(n - 1.0) + ((n - 1.0) * (n - 1.0) + 4.0 * kappa).sqrt()) / 2.0;
            let a = (sk / q) * (1.0 - dividend_discount * normal_cdf(d1));
            if spot < sk {
                Ok(gbs_price(&option_right, spot, strike, years, rate, carry, volatility)
                    + a * (spot / sk).powf(q))
            } else {
                Ok(spot - strike)
            }
        }
        OptionRight::Put => {
            let q = (-(n - 1.0) - ((n - 1.0) * (n - 1.0) + 4.0 * kappa).sqrt()) / 2.0;
            let a = -(sk / q) * (1.0 - dividend_discount * normal_cdf(-d1));
            if spot > sk {
                Ok(gbs_price(&option_right, spot, strike, years, rate, carry, volatility)
                    + a * (spot / sk).powf(q))
            } else {
                Ok(strike - spot)
            }
        }
    }
}

pub fn bjerksund_stensland_1993_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<f64> {
    let option_right =
        validate_inputs(spot, strike, years, rate, dividend_yield, volatility, option_right)?;
    let carry = carry(rate, dividend_yield);
    Ok(match option_right {
        OptionRight::Call => bs1993_call_price(spot, strike, years, rate, carry, volatility),
        OptionRight::Put => bs1993_call_price(strike, spot, years, dividend_yield, -carry, volatility),
    })
}

pub fn ju_quadratic_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<f64> {
    let option_right =
        validate_inputs(spot, strike, years, rate, dividend_yield, volatility, option_right)?;
    ju_quadratic_price_inner(
        &option_right,
        spot,
        strike,
        years,
        rate,
        dividend_yield,
        volatility,
    )
}

fn validate_cash_dividends(dividends: &[CashDividend], years: f64) -> OptionResult<Vec<CashDividend>> {
    if dividends.is_empty() {
        return Err(OptionError::new(
            "invalid_math_input",
            "dividends must not be empty",
        ));
    }

    let mut normalized = dividends.to_vec();
    normalized.sort_by(|left, right| left.time.total_cmp(&right.time));

    let mut rounded: Vec<CashDividend> = Vec::with_capacity(normalized.len());
    for dividend in normalized {
        ensure_positive("dividend.time", dividend.time)?;
        ensure_positive("dividend.amount", dividend.amount)?;
        let rounded_time = round_to_fixture_years(dividend.time);
        if rounded_time > years {
            return Err(OptionError::new(
                "invalid_math_input",
                format!("dividend.time exceeds years: {} > {}", rounded_time, years),
            ));
        }
        if let Some(last) = rounded.last_mut() {
            if (last.time - rounded_time).abs() <= 1e-12 {
                last.amount += dividend.amount;
                continue;
            }
        }
        rounded.push(CashDividend {
            time: rounded_time,
            amount: dividend.amount,
        });
    }

    Ok(rounded)
}

fn remaining_escrow_balance(dividends: &[CashDividend], rate: f64, time: f64) -> f64 {
    dividends
        .iter()
        .filter(|dividend| dividend.time + 1e-12 >= time)
        .map(|dividend| dividend.amount * (-rate * (dividend.time - time)).exp())
        .sum()
}

fn interpolate_linear(x_grid: &[f64], y_grid: &[f64], x: f64) -> f64 {
    if x <= x_grid[0] {
        return y_grid[0];
    }
    if x >= x_grid[x_grid.len() - 1] {
        return y_grid[y_grid.len() - 1];
    }

    let upper = x_grid.partition_point(|value| *value <= x);
    let lower = upper - 1;
    let x0 = x_grid[lower];
    let x1 = x_grid[upper];
    let y0 = y_grid[lower];
    let y1 = y_grid[upper];
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

fn actual_spot(
    state_spot: f64,
    rate: f64,
    time: f64,
    dividends: &[CashDividend],
    model: CashDividendModel,
) -> f64 {
    match model {
        CashDividendModel::Spot => state_spot,
        CashDividendModel::Escrowed => state_spot + remaining_escrow_balance(dividends, rate, time),
    }
}

fn dividend_step_value(
    x_grid: &[f64],
    values_after: &[f64],
    strike: f64,
    option_right: &OptionRight,
    rate: f64,
    dividend_time: f64,
    dividend_amount: f64,
    dividends: &[CashDividend],
    model: CashDividendModel,
) -> Vec<f64> {
    let balance = remaining_escrow_balance(dividends, rate, dividend_time);
    x_grid
        .iter()
        .map(|state_spot| {
            let continuation = match model {
                CashDividendModel::Spot => interpolate_linear(
                    x_grid,
                    values_after,
                    (state_spot - dividend_amount).max(x_grid[0]),
                ),
                CashDividendModel::Escrowed => interpolate_linear(x_grid, values_after, *state_spot),
            };
            let exercise = intrinsic(
                option_right,
                match model {
                    CashDividendModel::Spot => *state_spot,
                    CashDividendModel::Escrowed => state_spot + balance,
                },
                strike,
            );
            continuation.max(exercise)
        })
        .collect()
}

fn implicit_crank_nicolson_step(
    x_grid: &[f64],
    values: &[f64],
    dt: f64,
    strike: f64,
    option_right: &OptionRight,
    rate: f64,
    volatility: f64,
    dividends: &[CashDividend],
    model: CashDividendModel,
    time: f64,
) -> OptionResult<Vec<f64>> {
    let dx = x_grid[1] - x_grid[0];
    let last = x_grid.len() - 1;
    let balance = match model {
        CashDividendModel::Spot => 0.0,
        CashDividendModel::Escrowed => remaining_escrow_balance(dividends, rate, time),
    };
    let lower_boundary = intrinsic(
        option_right,
        match model {
            CashDividendModel::Spot => 0.0,
            CashDividendModel::Escrowed => balance,
        },
        strike,
    );
    let upper_boundary = intrinsic(option_right, x_grid[last] + balance, strike);

    let mut lower = vec![0.0; last - 1];
    let mut diag = vec![0.0; last - 1];
    let mut upper = vec![0.0; last - 1];
    let mut rhs = vec![0.0; last - 1];

    for index in 1..last {
        let state_spot = x_grid[index];
        let diffusion = 0.5 * volatility * volatility * state_spot * state_spot / (dx * dx);
        let drift = 0.5 * rate * state_spot / dx;
        let left = diffusion - drift;
        let center = -2.0 * diffusion - rate;
        let right = diffusion + drift;

        let row = index - 1;
        lower[row] = -0.5 * dt * left;
        diag[row] = 1.0 - 0.5 * dt * center;
        upper[row] = -0.5 * dt * right;
        rhs[row] = (1.0 + 0.5 * dt * center) * values[index]
            + 0.5 * dt * (left * values[index - 1] + right * values[index + 1]);
    }

    rhs[0] -= lower[0] * lower_boundary;
    rhs[last - 2] -= upper[last - 2] * upper_boundary;

    for row in 1..last - 1 {
        let weight = lower[row] / diag[row - 1];
        diag[row] -= weight * upper[row - 1];
        rhs[row] -= weight * rhs[row - 1];
    }

    let mut next = vec![0.0; x_grid.len()];
    next[0] = lower_boundary;
    next[last] = upper_boundary;
    next[last - 1] = rhs[last - 2] / diag[last - 2];

    for row in (0..last - 2).rev() {
        next[row + 1] = (rhs[row] - upper[row] * next[row + 2]) / diag[row];
    }

    for index in 1..last {
        let exercise = intrinsic(option_right, actual_spot(x_grid[index], rate, time, dividends, model), strike);
        next[index] = next[index].max(exercise);
    }

    Ok(next)
}

pub fn discrete_dividend_price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    volatility: f64,
    option_right: &str,
    cash_dividend_model: &str,
    dividends: &[CashDividend],
) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_positive("volatility", volatility)?;
    let option_right = parse_option_right(option_right)?;
    let model = CashDividendModel::parse(cash_dividend_model)?;
    let years = round_to_fixture_years(years);
    let dividends = validate_cash_dividends(dividends, years)?;

    let initial_balance = match model {
        CashDividendModel::Spot => 0.0,
        CashDividendModel::Escrowed => remaining_escrow_balance(&dividends, rate, 0.0),
    };
    let state_spot = spot - initial_balance;
    if state_spot <= 0.0 {
        return Err(OptionError::new(
            "invalid_math_input",
            "spot minus escrowed dividends must remain positive",
        ));
    }

    let reference_spot = spot
        .max(strike)
        .max(state_spot + remaining_escrow_balance(&dividends, rate, 0.0));
    let state_upper = (reference_spot * 4.0).max(1.0);
    let dx = state_upper / DISCRETE_DIVIDEND_SPACE_STEPS as f64;
    let x_grid = (0..=DISCRETE_DIVIDEND_SPACE_STEPS)
        .map(|index| index as f64 * dx)
        .collect::<Vec<_>>();

    let mut values = x_grid
        .iter()
        .map(|state| intrinsic(&option_right, *state, strike))
        .collect::<Vec<_>>();

    let mut event_times = dividends
        .iter()
        .filter(|dividend| dividend.time > 0.0 && dividend.time < years)
        .map(|dividend| dividend.time)
        .collect::<Vec<_>>();
    event_times.sort_by(|left, right| left.total_cmp(right));
    event_times.dedup_by(|left, right| (*left - *right).abs() <= 1e-12);

    let mut timeline = Vec::with_capacity(event_times.len() + 2);
    timeline.push(0.0);
    timeline.extend(event_times);
    timeline.push(years);

    for segment in (0..timeline.len() - 1).rev() {
        let start = timeline[segment];
        let end = timeline[segment + 1];

        if segment + 1 < timeline.len() - 1 {
            let dividend = dividends
                .iter()
                .find(|dividend| (dividend.time - end).abs() <= 1e-12)
                .expect("timeline dividend should exist");
            values = dividend_step_value(
                &x_grid,
                &values,
                strike,
                &option_right,
                rate,
                end,
                dividend.amount,
                &dividends,
                model,
            );
        }

        let sub_steps = ((end - start) * DISCRETE_DIVIDEND_TIME_STEPS_PER_YEAR)
            .round()
            .max(1.0) as usize;
        let dt = (end - start) / sub_steps as f64;

        for step in 0..sub_steps {
            let time = end - (step + 1) as f64 * dt;
            values = implicit_crank_nicolson_step(
                &x_grid,
                &values,
                dt,
                strike,
                &option_right,
                rate,
                volatility,
                &dividends,
                model,
                time,
            )?;
        }
    }

    Ok(interpolate_linear(&x_grid, &values, state_spot))
}
