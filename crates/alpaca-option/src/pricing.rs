use crate::error::{OptionError, OptionResult};
use crate::numeric::{brent_solve, normal_cdf, normal_pdf};
use crate::types::{
    BlackScholesImpliedVolatilityInput, BlackScholesInput, Greeks, OptionContract, OptionRight,
};

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_pricing_input",
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
            "invalid_pricing_input",
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

fn parse_option_right(option_right: &str) -> OptionResult<OptionRight> {
    OptionRight::from_str(option_right).map_err(|_| {
        OptionError::new(
            "invalid_pricing_input",
            format!("invalid option right: {option_right}"),
        )
    })
}

fn validate_black_scholes_inputs(input: &BlackScholesInput) -> OptionResult<()> {
    ensure_positive("spot", input.spot)?;
    ensure_positive("strike", input.strike)?;
    ensure_positive("years", input.years)?;
    ensure_finite("rate", input.rate)?;
    ensure_finite("dividend_yield", input.dividend_yield)?;
    ensure_positive("volatility", input.volatility)?;
    Ok(())
}

fn d1_d2(input: &BlackScholesInput) -> (f64, f64) {
    let sqrt_years = input.years.sqrt();
    let sigma_sqrt_t = input.volatility * sqrt_years;
    let d1 = ((input.spot / input.strike).ln()
        + (input.rate - input.dividend_yield + 0.5 * input.volatility * input.volatility)
            * input.years)
        / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    (d1, d2)
}

fn price_black_scholes_core(input: &BlackScholesInput) -> f64 {
    let (d1, d2) = d1_d2(input);
    let discount_spot = (-input.dividend_yield * input.years).exp();
    let discount_strike = (-input.rate * input.years).exp();

    match input.option_right {
        OptionRight::Call => {
            input.spot * discount_spot * normal_cdf(d1)
                - input.strike * discount_strike * normal_cdf(d2)
        }
        OptionRight::Put => {
            input.strike * discount_strike * normal_cdf(-d2)
                - input.spot * discount_spot * normal_cdf(-d1)
        }
    }
}

fn discounted_forward_minus_strike(input: &BlackScholesInput) -> f64 {
    input.spot * (-input.dividend_yield * input.years).exp()
        - input.strike * (-input.rate * input.years).exp()
}

fn european_no_arbitrage_lower_bound(input: &BlackScholesInput) -> f64 {
    let parity = discounted_forward_minus_strike(input);
    match input.option_right {
        OptionRight::Call => parity.max(0.0),
        OptionRight::Put => (-parity).max(0.0),
    }
}

pub fn price_black_scholes(input: &BlackScholesInput) -> OptionResult<f64> {
    validate_black_scholes_inputs(input)?;
    Ok(price_black_scholes_core(input))
}

pub fn greeks_black_scholes(input: &BlackScholesInput) -> OptionResult<Greeks> {
    validate_black_scholes_inputs(input)?;
    let (d1, d2) = d1_d2(input);
    let sqrt_years = input.years.sqrt();
    let sigma_sqrt_t = input.volatility * sqrt_years;
    let exp_minus_qt = (-input.dividend_yield * input.years).exp();
    let exp_minus_rt = (-input.rate * input.years).exp();
    let nd1 = normal_cdf(d1);
    let nd2 = normal_cdf(d2);
    let n_minus_d1 = normal_cdf(-d1);
    let n_minus_d2 = normal_cdf(-d2);
    let phi_d1 = normal_pdf(d1);

    let delta = match input.option_right {
        OptionRight::Call => exp_minus_qt * nd1,
        OptionRight::Put => -exp_minus_qt * n_minus_d1,
    };
    let gamma = exp_minus_qt * phi_d1 / (input.spot * sigma_sqrt_t);
    let vega = input.spot * exp_minus_qt * phi_d1 * sqrt_years / 100.0;
    let theta_annual = match input.option_right {
        OptionRight::Call => {
            -input.spot * exp_minus_qt * phi_d1 * input.volatility / (2.0 * sqrt_years)
                - input.rate * input.strike * exp_minus_rt * nd2
                + input.dividend_yield * input.spot * exp_minus_qt * nd1
        }
        OptionRight::Put => {
            -input.spot * exp_minus_qt * phi_d1 * input.volatility / (2.0 * sqrt_years)
                + input.rate * input.strike * exp_minus_rt * n_minus_d2
                - input.dividend_yield * input.spot * exp_minus_qt * n_minus_d1
        }
    };
    let theta = theta_annual / 365.0;
    let rho = match input.option_right {
        OptionRight::Call => input.strike * input.years * exp_minus_rt * nd2,
        OptionRight::Put => -input.strike * input.years * exp_minus_rt * n_minus_d2,
    };

    Ok(Greeks {
        delta,
        gamma,
        vega,
        theta,
        rho,
    })
}

pub fn intrinsic_value(spot: f64, strike: f64, option_right: &str) -> OptionResult<f64> {
    ensure_finite("spot", spot)?;
    ensure_positive("strike", strike)?;
    let option_right = parse_option_right(option_right)?;

    Ok(match option_right {
        OptionRight::Call => (spot - strike).max(0.0),
        OptionRight::Put => (strike - spot).max(0.0),
    })
}

pub fn extrinsic_value(option_price: f64, spot: f64, strike: f64, option_right: &str) -> OptionResult<f64> {
    ensure_finite("option_price", option_price)?;
    Ok((option_price - intrinsic_value(spot, strike, option_right)?).max(0.0))
}

pub fn contract_extrinsic_value(option_price: f64, spot: f64, contract: &OptionContract) -> OptionResult<f64> {
    extrinsic_value(option_price, spot, contract.strike, contract.option_right.as_str())
}

pub fn implied_volatility_from_price(
    input: &BlackScholesImpliedVolatilityInput,
) -> OptionResult<f64> {
    ensure_finite("target_price", input.target_price)?;
    let black_scholes_input = BlackScholesInput {
        spot: input.spot,
        strike: input.strike,
        years: input.years,
        rate: input.rate,
        dividend_yield: input.dividend_yield,
        volatility: 0.2,
        option_right: input.option_right.clone(),
    };
    validate_black_scholes_inputs(&black_scholes_input)?;
    let minimum_price = european_no_arbitrage_lower_bound(&black_scholes_input);
    if input.target_price + 1e-12 < minimum_price {
        return Err(OptionError::new(
            "invalid_pricing_input",
            format!(
                "target_price is below discounted no-arbitrage lower bound: {} < {minimum_price}",
                input.target_price
            ),
        ));
    }

    let lower_bound = input.lower_bound.unwrap_or(0.0001);
    let upper_bound = input.upper_bound.unwrap_or(5.0);
    ensure_positive("lower_bound", lower_bound)?;
    ensure_positive("upper_bound", upper_bound)?;
    if lower_bound >= upper_bound {
        return Err(OptionError::new(
            "invalid_pricing_input",
            format!("lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"),
        ));
    }

    brent_solve(
        lower_bound,
        upper_bound,
        |volatility| price_black_scholes_core(&BlackScholesInput {
            volatility,
            ..black_scholes_input.clone()
        }) - input.target_price,
        input.tolerance,
        input.max_iterations,
    )
}
