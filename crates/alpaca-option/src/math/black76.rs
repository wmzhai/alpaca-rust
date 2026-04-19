use crate::error::{OptionError, OptionResult};
use crate::numeric::{normal_cdf, normal_pdf};
use crate::types::{Greeks, OptionRight};

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

fn ensure_non_negative(name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(name, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_math_input",
            format!("{name} must be non-negative: {value}"),
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

fn sign(option_right: &OptionRight) -> f64 {
    match option_right {
        OptionRight::Call => 1.0,
        OptionRight::Put => -1.0,
    }
}

fn discount(rate: f64, years: f64) -> f64 {
    (-rate * years).exp()
}

fn intrinsic(option_right: &OptionRight, forward: f64, strike: f64) -> f64 {
    match option_right {
        OptionRight::Call => (forward - strike).max(0.0),
        OptionRight::Put => (strike - forward).max(0.0),
    }
}

fn validate_inputs(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<OptionRight> {
    ensure_positive("forward", forward)?;
    ensure_positive("strike", strike)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_non_negative("volatility", volatility)?;
    parse_option_right(option_right)
}

fn d1_d2(forward: f64, strike: f64, years: f64, volatility: f64) -> (f64, f64) {
    let sqrt_years = years.sqrt();
    let sigma_sqrt_t = volatility * sqrt_years;
    let d1 = ((forward / strike).ln() + 0.5 * volatility * volatility * years) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    (d1, d2)
}

fn price_core(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    volatility: f64,
    option_right: &OptionRight,
) -> f64 {
    let discount_r = discount(rate, years);
    if volatility == 0.0 {
        return discount_r * intrinsic(option_right, forward, strike);
    }

    let direction = sign(option_right);
    let (d1, d2) = d1_d2(forward, strike, years, volatility);
    discount_r
        * direction
        * (forward * normal_cdf(direction * d1) - strike * normal_cdf(direction * d2))
}

pub fn price(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<f64> {
    let option_right = validate_inputs(forward, strike, years, rate, volatility, option_right)?;
    Ok(price_core(
        forward,
        strike,
        years,
        rate,
        volatility,
        &option_right,
    ))
}

pub fn greeks(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    volatility: f64,
    option_right: &str,
) -> OptionResult<Greeks> {
    let option_right = validate_inputs(forward, strike, years, rate, volatility, option_right)?;
    ensure_positive("volatility", volatility)?;

    let discount_r = discount(rate, years);
    let sqrt_years = years.sqrt();
    let sigma_sqrt_t = volatility * sqrt_years;
    let (d1, _) = d1_d2(forward, strike, years, volatility);
    let pdf_d1 = normal_pdf(d1);
    let price = price_core(forward, strike, years, rate, volatility, &option_right);

    let delta = match option_right {
        OptionRight::Call => discount_r * normal_cdf(d1),
        OptionRight::Put => discount_r * (normal_cdf(d1) - 1.0),
    };
    let gamma = discount_r * pdf_d1 / (forward * sigma_sqrt_t);
    let vega = discount_r * forward * pdf_d1 * sqrt_years;
    let theta = rate * price - discount_r * forward * pdf_d1 * volatility / (2.0 * sqrt_years);
    let rho = -years * price;

    Ok(Greeks {
        delta,
        gamma,
        vega,
        theta,
        rho,
    })
}

pub fn implied_volatility_from_price(
    target_price: f64,
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    option_right: &str,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<f64> {
    ensure_finite("target_price", target_price)?;
    let option_right = validate_inputs(forward, strike, years, rate, 0.2, option_right)?;
    let intrinsic_price = discount(rate, years) * intrinsic(&option_right, forward, strike);
    if target_price < intrinsic_price {
        return Err(OptionError::new(
            "invalid_math_input",
            format!("target_price is below discounted intrinsic value: {target_price} < {intrinsic_price}"),
        ));
    }

    let lower_bound = lower_bound.unwrap_or(0.0001);
    let upper_bound = upper_bound.unwrap_or(5.0);
    ensure_positive("lower_bound", lower_bound)?;
    ensure_positive("upper_bound", upper_bound)?;
    if lower_bound >= upper_bound {
        return Err(OptionError::new(
            "invalid_math_input",
            format!("lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"),
        ));
    }

    let tolerance = tolerance.unwrap_or(1e-10);
    ensure_positive("tolerance", tolerance)?;
    let max_iterations = max_iterations.unwrap_or(128);
    if max_iterations == 0 {
        return Err(OptionError::new(
            "invalid_math_input",
            "max_iterations must be greater than zero",
        ));
    }

    let mut lower = lower_bound;
    let mut upper = upper_bound;
    let mut lower_value = price_core(forward, strike, years, rate, lower, &option_right) - target_price;
    let mut upper_value = price_core(forward, strike, years, rate, upper, &option_right) - target_price;
    if lower_value.abs() <= tolerance {
        return Ok(lower);
    }
    if upper_value.abs() <= tolerance {
        return Ok(upper);
    }
    if lower_value * upper_value > 0.0 {
        return Err(OptionError::new(
            "root_not_bracketed",
            format!("root is not bracketed: f({lower})={lower_value}, f({upper})={upper_value}"),
        ));
    }

    let mut volatility = (lower + upper) / 2.0;
    for _ in 0..max_iterations {
        let value = price_core(forward, strike, years, rate, volatility, &option_right) - target_price;
        let vega = discount(rate, years)
            * forward
            * normal_pdf(d1_d2(forward, strike, years, volatility).0)
            * years.sqrt();
        let step_estimate = if vega.is_finite() && vega > 0.0 {
            (value / vega).abs()
        } else {
            f64::INFINITY
        };
        if (value.abs() <= tolerance && step_estimate <= tolerance) || (upper - lower).abs() <= tolerance {
            return Ok(volatility);
        }
        if value < 0.0 {
            lower = volatility;
            lower_value = value;
        } else {
            upper = volatility;
            upper_value = value;
        }

        let newton_candidate = if vega.is_finite() && vega > 0.0 {
            volatility - value / vega
        } else {
            f64::NAN
        };
        if newton_candidate.is_finite() && newton_candidate > lower && newton_candidate < upper {
            volatility = newton_candidate;
        } else if lower_value.abs() < upper_value.abs() {
            volatility = (lower + upper) / 2.0;
        } else {
            volatility = (lower + upper) / 2.0;
        }
    }

    Ok(volatility)
}
