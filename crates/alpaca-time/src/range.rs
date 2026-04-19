use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::calendar::{add_trading_days, is_trading_date_naive};
use crate::clock::parse_naive_date;
use crate::error::{TimeError, TimeResult};
use crate::types::DateRange;

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_weekday_code(weekday: &str) -> TimeResult<Weekday> {
    match weekday {
        "mon" => Ok(Weekday::Mon),
        "tue" => Ok(Weekday::Tue),
        "wed" => Ok(Weekday::Wed),
        "thu" => Ok(Weekday::Thu),
        "fri" => Ok(Weekday::Fri),
        "sat" => Ok(Weekday::Sat),
        "sun" => Ok(Weekday::Sun),
        _ => Err(TimeError::new(
            "invalid_weekday",
            format!("invalid weekday: {weekday}"),
        )),
    }
}

pub fn add_days(date: &str, days: i32) -> TimeResult<String> {
    let date = parse_naive_date(date)?;
    Ok(format_date(date + Duration::days(days as i64)))
}

pub fn dates(start_date: &str, end_date: &str) -> TimeResult<Vec<String>> {
    let start = parse_naive_date(start_date)?;
    let end = parse_naive_date(end_date)?;

    if start > end {
        return Err(TimeError::new(
            "invalid_date_range",
            format!("start_date must be <= end_date: {start_date} > {end_date}"),
        ));
    }

    let mut values = Vec::new();
    let mut current = start;
    while current <= end {
        values.push(format_date(current));
        current = current
            .succ_opt()
            .ok_or_else(|| TimeError::new("date_overflow", "date overflow while building list"))?;
    }

    Ok(values)
}

pub fn trading_dates(start_date: &str, end_date: &str) -> TimeResult<Vec<String>> {
    Ok(dates(start_date, end_date)?
        .into_iter()
        .filter(|date| parse_naive_date(date).map(is_trading_date_naive).unwrap_or(false))
        .collect())
}

pub fn nth_weekday(year: i32, month: u32, weekday: &str, nth: u32) -> TimeResult<String> {
    if !(1..=12).contains(&month) || nth == 0 {
        return Err(TimeError::new(
            "invalid_weekday_selection",
            format!("invalid nth weekday input: year={year}, month={month}, nth={nth}"),
        ));
    }

    let target_weekday = parse_weekday_code(weekday)?;
    let mut current = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        TimeError::new(
            "invalid_weekday_selection",
            format!("invalid nth weekday input: year={year}, month={month}, nth={nth}"),
        )
    })?;

    while current.weekday() != target_weekday {
        current = current
            .succ_opt()
            .ok_or_else(|| TimeError::new("date_overflow", "date overflow while selecting weekday"))?;
    }

    current += Duration::days(((nth - 1) * 7) as i64);
    if current.month() != month {
        return Err(TimeError::new(
            "invalid_weekday_selection",
            format!("weekday {weekday} #{nth} does not exist in {year}-{month:02}"),
        ));
    }

    Ok(format_date(current))
}

pub fn is_last_trading_date_of_week(date: &str) -> bool {
    let Ok(date) = parse_naive_date(date) else {
        return false;
    };
    if !is_trading_date_naive(date) {
        return false;
    }

    let Ok(next_trading) = add_trading_days(&format_date(date), 1)
        .and_then(|value| parse_naive_date(&value)) else {
        return false;
    };
    let current_week_start = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let next_week_start =
        next_trading - Duration::days(next_trading.weekday().num_days_from_monday() as i64);
    current_week_start != next_week_start
}

pub fn weekly_last_trading_dates(start_date: &str, end_date: &str) -> TimeResult<Vec<String>> {
    let mut values = Vec::new();
    for date in trading_dates(start_date, end_date)? {
        if is_last_trading_date_of_week(&date) {
            values.push(date);
        }
    }
    Ok(values)
}

pub fn last_trading_date_of_week(date: &str) -> TimeResult<String> {
    let week = calendar_week_range(date)?;
    weekly_last_trading_dates(&week.start_date, &week.end_date)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            TimeError::new(
                "missing_last_trading_date_of_week",
                format!("no trading day found for week containing {date}"),
            )
        })
}

pub fn calendar_week_range(date: &str) -> TimeResult<DateRange> {
    let date = parse_naive_date(date)?;
    let start = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let end = start + Duration::days(6);
    Ok(DateRange {
        start_date: format_date(start),
        end_date: format_date(end),
    })
}

pub fn iso_week_range(year: i32, week: u32) -> TimeResult<DateRange> {
    let start = NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).ok_or_else(|| {
        TimeError::new(
            "invalid_iso_week",
            format!("invalid iso week: year={year}, week={week}"),
        )
    })?;
    let end = start + Duration::days(6);
    Ok(DateRange {
        start_date: format_date(start),
        end_date: format_date(end),
    })
}
