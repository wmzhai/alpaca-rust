use crate::error::{OptionError, OptionResult};
use crate::numeric::normal_cdf;

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
    let d2_lower = ((spot / lower_price).ln() + (rate - dividend_yield - 0.5 * volatility * volatility) * years)
        / sigma_sqrt_t;
    let d2_upper = ((spot / upper_price).ln() + (rate - dividend_yield - 0.5 * volatility * volatility) * years)
        / sigma_sqrt_t;

    Ok(normal_cdf(d2_lower) - normal_cdf(d2_upper))
}
