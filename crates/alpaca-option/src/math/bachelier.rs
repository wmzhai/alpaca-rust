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

fn solver_residual(
    option_right: &OptionRight,
    target_price: f64,
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    normal_volatility: f64,
) -> f64 {
    let intrinsic_price = discount(rate, years) * intrinsic(option_right, forward, strike);
    match option_right {
        OptionRight::Call if forward > strike => {
            intrinsic_price
                + price_core(
                    forward,
                    strike,
                    years,
                    rate,
                    normal_volatility,
                    &OptionRight::Put,
                )
                - target_price
        }
        OptionRight::Put if forward < strike => {
            intrinsic_price
                + price_core(
                    forward,
                    strike,
                    years,
                    rate,
                    normal_volatility,
                    &OptionRight::Call,
                )
                - target_price
        }
        _ => price_core(
            forward,
            strike,
            years,
            rate,
            normal_volatility,
            option_right,
        ) - target_price,
    }
}

fn validate_inputs(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    normal_volatility: f64,
    option_right: &str,
) -> OptionResult<OptionRight> {
    ensure_finite("forward", forward)?;
    ensure_finite("strike", strike)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_non_negative("normal_volatility", normal_volatility)?;
    parse_option_right(option_right)
}

fn d_value(forward: f64, strike: f64, years: f64, normal_volatility: f64) -> f64 {
    (forward - strike) / (normal_volatility * years.sqrt())
}

fn price_core(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    normal_volatility: f64,
    option_right: &OptionRight,
) -> f64 {
    let discount_r = discount(rate, years);
    if normal_volatility == 0.0 {
        return discount_r * intrinsic(option_right, forward, strike);
    }

    let direction = sign(option_right);
    let spread = forward - strike;
    let std_dev = normal_volatility * years.sqrt();
    let d = spread / std_dev;

    discount_r
        * (direction * spread * normal_cdf(direction * d) + std_dev * normal_pdf(d))
}

pub fn price(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    normal_volatility: f64,
    option_right: &str,
) -> OptionResult<f64> {
    let option_right =
        validate_inputs(forward, strike, years, rate, normal_volatility, option_right)?;
    Ok(price_core(
        forward,
        strike,
        years,
        rate,
        normal_volatility,
        &option_right,
    ))
}

pub fn greeks(
    forward: f64,
    strike: f64,
    years: f64,
    rate: f64,
    normal_volatility: f64,
    option_right: &str,
) -> OptionResult<Greeks> {
    let option_right =
        validate_inputs(forward, strike, years, rate, normal_volatility, option_right)?;
    ensure_positive("normal_volatility", normal_volatility)?;

    let discount_r = discount(rate, years);
    let sqrt_years = years.sqrt();
    let d = d_value(forward, strike, years, normal_volatility);
    let pdf_d = normal_pdf(d);
    let price = price_core(
        forward,
        strike,
        years,
        rate,
        normal_volatility,
        &option_right,
    );

    let delta = match option_right {
        OptionRight::Call => discount_r * normal_cdf(d),
        OptionRight::Put => discount_r * (normal_cdf(d) - 1.0),
    };
    let gamma = discount_r * pdf_d / (normal_volatility * sqrt_years);
    let vega = discount_r * sqrt_years * pdf_d;
    let theta = rate * price - discount_r * normal_volatility * pdf_d / (2.0 * sqrt_years);
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
    let option_right = validate_inputs(forward, strike, years, rate, 0.0, option_right)?;
    let intrinsic_price = discount(rate, years) * intrinsic(&option_right, forward, strike);
    if target_price < intrinsic_price {
        return Err(OptionError::new(
            "invalid_math_input",
            format!(
                "target_price is below discounted intrinsic value: {target_price} < {intrinsic_price}"
            ),
        ));
    }
    let lower_bound = lower_bound.unwrap_or(0.0);
    let upper_bound = upper_bound.unwrap_or(20.0);
    ensure_non_negative("lower_bound", lower_bound)?;
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
    let mut lower_value =
        solver_residual(&option_right, target_price, forward, strike, years, rate, lower);
    let mut upper_value =
        solver_residual(&option_right, target_price, forward, strike, years, rate, upper);
    if lower_value == 0.0 {
        return Ok(lower);
    }
    if upper_value == 0.0 {
        return Ok(upper);
    }
    if lower_value * upper_value > 0.0 {
        return Err(OptionError::new(
            "root_not_bracketed",
            format!("root is not bracketed: f({lower})={lower_value}, f({upper})={upper_value}"),
        ));
    }

    let mut normal_volatility = (lower + upper) / 2.0;
    for _ in 0..max_iterations {
        let value = solver_residual(
            &option_right,
            target_price,
            forward,
            strike,
            years,
            rate,
            normal_volatility,
        );
        if value == 0.0 {
            return Ok(normal_volatility);
        }
        if value < 0.0 {
            lower = normal_volatility;
            lower_value = value;
        } else {
            upper = normal_volatility;
            upper_value = value;
        }
        if lower_value == 0.0 {
            return Ok(lower);
        }
        if upper_value == 0.0 {
            return Ok(upper);
        }

        normal_volatility = (lower + upper) / 2.0;
        if (upper - lower).abs() <= tolerance {
            return Ok(normal_volatility);
        }
    }

    Ok(normal_volatility)
}
