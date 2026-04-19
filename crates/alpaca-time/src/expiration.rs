use crate::calendar::{is_trading_date, market_hours_for_date};
use crate::clock::{now, parse_naive_date, parse_naive_timestamp, parse_timestamp};
use crate::error::{TimeError, TimeResult};
use crate::types::DayCountBasis;

pub fn close(expiration_date: &str) -> TimeResult<String> {
    if !is_trading_date(expiration_date) {
        return Err(TimeError::new(
            "invalid_expiration_date",
            format!("expiration date must be a trading date: {expiration_date}"),
        ));
    }
    let hours = market_hours_for_date(expiration_date)?;
    Ok(format!(
        "{} {}:00",
        expiration_date,
        hours
            .regular_close
            .ok_or_else(|| TimeError::new("invalid_market_hours", "missing regular close"))?
    ))
}

pub fn calendar_days(expiration_date: &str, at: Option<&str>) -> TimeResult<i64> {
    let timestamp = parse_naive_timestamp(&parse_timestamp(at.unwrap_or(&now()))?)?;
    let start_date = timestamp.date();
    let expiration_date = parse_naive_date(expiration_date)?;
    let day_diff = (expiration_date - start_date).num_days();
    if day_diff != 0 {
        return Ok(day_diff);
    }
    let expiration = parse_naive_timestamp(&close(&expiration_date.format("%Y-%m-%d").to_string())?)?;
    if timestamp <= expiration {
        Ok(0)
    } else {
        Ok(-1)
    }
}

pub fn days(expiration_date: &str, at: Option<&str>) -> TimeResult<f64> {
    let start = parse_naive_timestamp(&parse_timestamp(at.unwrap_or(&now()))?)?;
    let end = parse_naive_timestamp(&close(expiration_date)?)?;
    let seconds = end.signed_duration_since(start).num_seconds();
    Ok(seconds as f64 / 86_400.0)
}

pub fn years(expiration_date: &str, at: Option<&str>, basis: Option<&str>) -> f64 {
    let basis = DayCountBasis::from_option_str(basis);
    days(expiration_date, at)
        .map(|value| (value / basis.denominator()).max(0.0))
        .unwrap_or(0.0)
}

pub fn years_between_dates(start_date: &str, end_date: &str, basis: Option<&str>) -> TimeResult<f64> {
    let start_date = parse_naive_date(start_date)?;
    let end_date = parse_naive_date(end_date)?;
    let basis = match basis {
        Some(value) => DayCountBasis::from_option_str(Some(value)),
        None => DayCountBasis::Act365,
    };
    Ok((end_date - start_date).num_days() as f64 / basis.denominator())
}
