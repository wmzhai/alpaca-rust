use crate::error::{OptionError, OptionResult};
use crate::numeric::normal_cdf;
use crate::types::OptionRight;

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

pub fn price(
    spot: f64,
    strike: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
    average_style: &str,
) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_finite("dividend_yield", dividend_yield)?;
    ensure_non_negative("volatility", volatility)?;
    let option_right = parse_option_right(option_right)?;

    if average_style != "continuous" {
        return Err(OptionError::new(
            "unsupported_math_input",
            format!("unsupported average_style: {average_style}"),
        ));
    }

    let sign = match option_right {
        OptionRight::Call => 1.0,
        OptionRight::Put => -1.0,
    };
    let variance = volatility * volatility * years / 3.0;

    if variance == 0.0 {
        let mean_level = spot * ((rate - dividend_yield) * years / 2.0).exp();
        return Ok((-rate * years).exp() * (sign * (mean_level - strike)).max(0.0));
    }

    let mean_ln = spot.ln() + (rate - dividend_yield - 0.5 * volatility * volatility) * years / 2.0;
    let std_dev = variance.sqrt();
    let d1 = (mean_ln - strike.ln() + variance) / std_dev;
    let d2 = d1 - std_dev;
    let expected_average = (mean_ln + 0.5 * variance).exp();

    Ok((-rate * years).exp()
        * sign
        * (expected_average * normal_cdf(sign * d1) - strike * normal_cdf(sign * d2)))
}
