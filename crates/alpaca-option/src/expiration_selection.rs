use alpaca_time::clock;
use alpaca_time::range;

use crate::error::{OptionError, OptionResult};

fn canonical_date(date: &str) -> OptionResult<String> {
    clock::parse_date(date).map_err(|_| {
        OptionError::new(
            "invalid_expiration_selection_input",
            format!("invalid date: {date}"),
        )
    })
}

pub fn nearest_weekly_expiration(anchor_date: &str) -> OptionResult<String> {
    let anchor = canonical_date(anchor_date)?;
    let end = range::add_days(&anchor, 14).map_err(|error| {
        OptionError::new("invalid_expiration_selection_input", error.to_string())
    })?;
    let candidates = range::weekly_last_trading_dates(&anchor, &end).map_err(|error| {
        OptionError::new("invalid_expiration_selection_input", error.to_string())
    })?;
    candidates.into_iter().next().ok_or_else(|| {
        OptionError::new(
            "missing_weekly_expiration",
            format!("no weekly expiration found on or after {anchor_date}"),
        )
    })
}

pub fn weekly_expirations_between(start_date: &str, end_date: &str) -> OptionResult<Vec<String>> {
    let start_date = canonical_date(start_date)?;
    let end_date = canonical_date(end_date)?;
    range::weekly_last_trading_dates(&start_date, &end_date)
        .map_err(|error| OptionError::new("invalid_expiration_selection_input", error.to_string()))
}

pub fn standard_monthly_expiration(year: i32, month: u32) -> OptionResult<String> {
    let third_friday = range::nth_weekday(year, month, "fri", 3).map_err(|error| {
        OptionError::new("invalid_expiration_selection_input", error.to_string())
    })?;
    range::last_trading_date_of_week(&third_friday)
        .map_err(|error| OptionError::new("invalid_expiration_selection_input", error.to_string()))
}
