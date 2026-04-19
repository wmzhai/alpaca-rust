use chrono::{Datelike, Duration, NaiveDate, Timelike, Weekday};
use std::collections::HashSet;

use crate::clock::{minutes_from_hhmm, now, parse_naive_date, parse_naive_timestamp, today};
use crate::error::{TimeError, TimeResult};
use crate::types::{MarketHours, TradingDayInfo};

fn observed_fixed_holiday(year: i32, month: u32, day: u32) -> NaiveDate {
    let date = NaiveDate::from_ymd_opt(year, month, day).expect("valid fixed holiday");
    match date.weekday() {
        Weekday::Sat => date - Duration::days(1),
        Weekday::Sun => date + Duration::days(1),
        _ => date,
    }
}

fn nth_weekday(year: i32, month: u32, weekday: Weekday, nth: u32) -> NaiveDate {
    let mut current = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    while current.weekday() != weekday {
        current += Duration::days(1);
    }
    current + Duration::days(((nth - 1) * 7) as i64)
}

fn last_weekday(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    let mut current = next_month - Duration::days(1);
    while current.weekday() != weekday {
        current -= Duration::days(1);
    }
    current
}

fn easter_date(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

fn holiday_dates_for_year(year: i32) -> HashSet<NaiveDate> {
    let mut holidays = HashSet::new();
    holidays.insert(observed_fixed_holiday(year, 1, 1));
    holidays.insert(nth_weekday(year, 1, Weekday::Mon, 3));
    holidays.insert(nth_weekday(year, 2, Weekday::Mon, 3));
    holidays.insert(easter_date(year) - Duration::days(2));
    holidays.insert(last_weekday(year, 5, Weekday::Mon));
    holidays.insert(observed_fixed_holiday(year, 6, 19));
    holidays.insert(observed_fixed_holiday(year, 7, 4));
    holidays.insert(nth_weekday(year, 9, Weekday::Mon, 1));
    holidays.insert(nth_weekday(year, 11, Weekday::Thu, 4));
    holidays.insert(observed_fixed_holiday(year, 12, 25));
    holidays
}

fn early_close_dates_for_year(year: i32) -> HashSet<NaiveDate> {
    let mut early_close = HashSet::new();

    let thanksgiving = nth_weekday(year, 11, Weekday::Thu, 4);
    let black_friday = thanksgiving + Duration::days(1);
    if is_weekday(black_friday) && !holiday_dates_for_year(year).contains(&black_friday) {
        early_close.insert(black_friday);
    }

    let independence_holiday = observed_fixed_holiday(year, 7, 4);
    let independence_eve = add_trading_days_from(independence_holiday, -1);
    if independence_eve.year() == year {
        early_close.insert(independence_eve);
    }

    if let Some(christmas_eve) = NaiveDate::from_ymd_opt(year, 12, 24) {
        if is_weekday(christmas_eve) && !holiday_dates_for_year(year).contains(&christmas_eve) {
            early_close.insert(christmas_eve);
        }
    }

    early_close
}

fn is_weekday(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

fn add_trading_days_from(mut date: NaiveDate, days: i32) -> NaiveDate {
    let step = match days.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => return date,
        std::cmp::Ordering::Greater => 1,
    };
    let mut remaining = days.abs();
    while remaining > 0 {
        date += Duration::days(step as i64);
        if is_trading_date_naive(date) {
            remaining -= 1;
        }
    }
    date
}

pub(crate) fn is_market_holiday_naive(date: NaiveDate) -> bool {
    holiday_dates_for_year(date.year()).contains(&date)
}

pub(crate) fn is_early_close_naive(date: NaiveDate) -> bool {
    early_close_dates_for_year(date.year()).contains(&date)
}

pub(crate) fn is_trading_date_naive(date: NaiveDate) -> bool {
    is_weekday(date) && !is_market_holiday_naive(date)
}

pub fn trading_day_info(date: &str) -> TimeResult<TradingDayInfo> {
    let date = parse_naive_date(date)?;
    let is_trading_date = is_trading_date_naive(date);
    let is_market_holiday = is_market_holiday_naive(date);
    let is_early_close = is_trading_date && is_early_close_naive(date);
    let market_hours = market_hours_for_date(&date.format("%Y-%m-%d").to_string())?;

    Ok(TradingDayInfo {
        date: date.format("%Y-%m-%d").to_string(),
        is_trading_date,
        is_market_holiday,
        is_early_close,
        market_hours,
    })
}

pub fn is_trading_date(date: &str) -> bool {
    parse_naive_date(date).map(is_trading_date_naive).unwrap_or(false)
}

pub fn is_trading_today() -> bool {
    is_trading_date(&today())
}

pub fn market_hours_for_date(date: &str) -> TimeResult<MarketHours> {
    let date = parse_naive_date(date)?;
    let date_string = date.format("%Y-%m-%d").to_string();
    let is_trading_date = is_trading_date_naive(date);
    let is_early_close = is_trading_date && is_early_close_naive(date);

    if !is_trading_date {
        return Ok(MarketHours {
            date: date_string,
            is_trading_date: false,
            is_early_close: false,
            premarket_open: None,
            regular_open: None,
            regular_close: None,
            after_hours_close: None,
        });
    }

    Ok(MarketHours {
        date: date_string,
        is_trading_date: true,
        is_early_close,
        premarket_open: Some("04:00".to_string()),
        regular_open: Some("09:30".to_string()),
        regular_close: Some(if is_early_close { "13:00" } else { "16:00" }.to_string()),
        after_hours_close: Some("20:00".to_string()),
    })
}

pub fn last_completed_trading_date(at_timestamp: Option<&str>) -> TimeResult<String> {
    let timestamp = parse_naive_timestamp(at_timestamp.unwrap_or(&now()))?;
    let date_string = timestamp.format("%Y-%m-%d").to_string();
    let hours = market_hours_for_date(&date_string)?;

    if !hours.is_trading_date {
        return add_trading_days(&date_string, -1);
    }

    let regular_close = hhmm_to_total_minutes(
        hours
            .regular_close
            .as_deref()
            .ok_or_else(|| TimeError::new("invalid_market_hours", "missing regular close"))?,
    )?;
    let current_minutes = timestamp.hour() * 60 + timestamp.minute();

    if current_minutes >= regular_close {
        Ok(date_string)
    } else {
        add_trading_days(&date_string, -1)
    }
}

pub fn add_trading_days(date: &str, days: i32) -> TimeResult<String> {
    let date = parse_naive_date(date)?;
    Ok(add_trading_days_from(date, days)
        .format("%Y-%m-%d")
        .to_string())
}

pub(crate) fn hhmm_to_total_minutes(hhmm: &str) -> TimeResult<u32> {
    minutes_from_hhmm(hhmm)
}
