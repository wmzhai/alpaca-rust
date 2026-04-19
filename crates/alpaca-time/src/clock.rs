use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::America::New_York;
use std::cmp::Ordering;

use crate::error::{TimeError, TimeResult};
use crate::types::TimestampParts;

pub(crate) fn parse_naive_date(input: &str) -> TimeResult<NaiveDate> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| TimeError::new("invalid_date", format!("invalid date: {input}")))
}

pub(crate) fn parse_naive_timestamp(input: &str) -> TimeResult<NaiveDateTime> {
    if let Ok(timestamp) = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Ok(timestamp);
    }

    if let Ok(rfc3339) = DateTime::parse_from_rfc3339(input) {
        let ny_local = rfc3339.with_timezone(&New_York).naive_local();
        return ny_local
            .with_nanosecond(0)
            .ok_or_else(|| TimeError::new("invalid_timestamp", format!("invalid timestamp: {input}")));
    }

    Err(TimeError::new("invalid_timestamp", format!("invalid timestamp: {input}")))
}

pub(crate) fn format_naive_timestamp(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn comparable_input_to_naive_timestamp(input: &str) -> TimeResult<NaiveDateTime> {
    if let Ok(date) = parse_naive_date(input) {
        return date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| TimeError::new("invalid_date", format!("invalid date: {input}")));
    }
    parse_naive_timestamp(input)
}

pub(crate) fn ny_local_to_utc(value: NaiveDateTime) -> TimeResult<DateTime<Utc>> {
    let localized = New_York
        .from_local_datetime(&value)
        .single()
        .or_else(|| New_York.from_local_datetime(&value).earliest())
        .or_else(|| New_York.from_local_datetime(&value).latest())
        .ok_or_else(|| TimeError::new("invalid_ny_local_time", format!("cannot localize NY time: {value}")))?;
    Ok(localized.with_timezone(&Utc))
}

pub fn now() -> String {
    Utc::now()
        .with_timezone(&New_York)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn today() -> String {
    Utc::now()
        .with_timezone(&New_York)
        .format("%Y-%m-%d")
        .to_string()
}

pub fn parse_date(input: &str) -> TimeResult<String> {
    Ok(parse_naive_date(input)?.format("%Y-%m-%d").to_string())
}

pub fn parse_timestamp(input: &str) -> TimeResult<String> {
    Ok(format_naive_timestamp(parse_naive_timestamp(input)?))
}

pub fn parts(input: Option<&str>) -> TimeResult<TimestampParts> {
    let timestamp = parse_naive_timestamp(input.unwrap_or(&now()))?;
    Ok(TimestampParts {
        date: timestamp.format("%Y-%m-%d").to_string(),
        timestamp: format_naive_timestamp(timestamp),
        year: timestamp.year(),
        month: timestamp.month(),
        day: timestamp.day(),
        hour: timestamp.hour(),
        minute: timestamp.minute(),
        second: timestamp.second(),
        hhmm: timestamp.hour() * 100 + timestamp.minute(),
        hhmm_string: format!("{:02}:{:02}", timestamp.hour(), timestamp.minute()),
        weekday_from_sunday: timestamp.weekday().num_days_from_sunday(),
    })
}

pub fn parse_date_or_timestamp(input: &str) -> TimeResult<String> {
    parse_date(input).or_else(|_| parse_timestamp(input))
}

pub fn first_date_or_timestamp<'a, I>(inputs: I) -> Option<String>
where
    I: IntoIterator<Item = Option<&'a str>>,
{
    for input in inputs {
        let Some(input) = input else {
            continue;
        };

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(value) = parse_date_or_timestamp(trimmed) {
            return Some(value);
        }
    }

    None
}

pub fn to_utc_rfc3339(input: &str) -> TimeResult<String> {
    let utc = ny_local_to_utc(parse_naive_timestamp(input)?)?;
    Ok(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

pub fn from_unix_seconds(seconds: i64) -> String {
    if seconds == 0 {
        return String::new();
    }

    Utc.timestamp_opt(seconds, 0)
        .single()
        .map(|utc| {
            utc.with_timezone(&New_York)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}

pub fn truncate_to_minute(input: &str) -> String {
    parse_naive_timestamp(input)
        .and_then(|timestamp| {
            timestamp
                .with_second(0)
                .ok_or_else(|| TimeError::new("invalid_timestamp", "failed to truncate second"))
                .map(format_naive_timestamp)
        })
        .unwrap_or_else(|_| {
            if input.len() >= 16 {
                format!("{}:00", &input[0..16])
            } else {
                input.to_string()
            }
        })
}

pub fn minute_key(input: &str) -> TimeResult<String> {
    let timestamp = parse_naive_timestamp(input)?;
    Ok(timestamp.format("%Y-%m-%d %H:%M").to_string())
}

pub fn hhmm_string_from_parts(hour: u32, minute: u32) -> TimeResult<String> {
    if hour > 23 || minute > 59 {
        return Err(TimeError::new(
            "invalid_hhmm_parts",
            format!("invalid time parts: {hour}:{minute}"),
        ));
    }
    Ok(format!("{hour:02}:{minute:02}"))
}

pub fn minutes_from_hhmm(input: &str) -> TimeResult<u32> {
    let mut pieces = input.split(':');
    let hour = pieces
        .next()
        .ok_or_else(|| TimeError::new("invalid_hhmm", format!("invalid hhmm: {input}")))?
        .parse::<u32>()
        .map_err(|_| TimeError::new("invalid_hhmm", format!("invalid hhmm: {input}")))?;
    let minute = pieces
        .next()
        .ok_or_else(|| TimeError::new("invalid_hhmm", format!("invalid hhmm: {input}")))?
        .parse::<u32>()
        .map_err(|_| TimeError::new("invalid_hhmm", format!("invalid hhmm: {input}")))?;
    if pieces.next().is_some() || hour > 23 || minute > 59 {
        return Err(TimeError::new("invalid_hhmm", format!("invalid hhmm: {input}")));
    }
    Ok(hour * 60 + minute)
}

fn parsed_date_or_timestamp_parts(input: &str) -> TimeResult<(String, Option<String>)> {
    if let Ok(timestamp) = parse_timestamp(input) {
        return Ok((timestamp[0..10].to_string(), Some(timestamp)));
    }

    Ok((parse_date(input)?, None))
}

pub fn compare_date_or_timestamp(left: &str, right: &str) -> TimeResult<Ordering> {
    let left = left.trim();
    let right = right.trim();

    if left.is_empty() || right.is_empty() {
        return Ok(left.cmp(right));
    }

    let (left_date, left_timestamp) = parsed_date_or_timestamp_parts(left)?;
    let (right_date, right_timestamp) = parsed_date_or_timestamp_parts(right)?;
    let date_cmp = left_date.cmp(&right_date);
    if date_cmp != Ordering::Equal {
        return Ok(date_cmp);
    }

    match (left_timestamp, right_timestamp) {
        (Some(left_timestamp), Some(right_timestamp)) => Ok(left_timestamp.cmp(&right_timestamp)),
        _ => Ok(Ordering::Equal),
    }
}

pub fn fractional_days_between(start: &str, end: &str) -> TimeResult<f64> {
    let start = comparable_input_to_naive_timestamp(start)?;
    let end = comparable_input_to_naive_timestamp(end)?;
    let seconds = end.signed_duration_since(start).num_seconds();
    Ok(seconds as f64 / 86_400.0)
}

pub fn fractional_days_until(target: &str) -> TimeResult<f64> {
    fractional_days_between(&now(), target)
}

pub fn fractional_days_since(input: &str) -> TimeResult<f64> {
    fractional_days_between(input, &now())
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use ::chrono::{NaiveDate, NaiveDateTime};
    use crate::chrono;

    use super::{
        compare_date_or_timestamp, first_date_or_timestamp, now, parse_date_or_timestamp,
        parse_timestamp, parts, today, truncate_to_minute,
    };

    #[test]
    fn accepts_rfc3339_utc_timestamps() {
        assert_eq!(
            parse_timestamp("2026-04-12T02:35:16Z").unwrap(),
            "2026-04-11 22:35:16"
        );
        assert_eq!(
            truncate_to_minute("2026-04-12T02:35:16Z"),
            "2026-04-11 22:35:00"
        );
    }

    #[test]
    fn parse_date_or_timestamp_preserves_input_granularity() {
        assert_eq!(parse_date_or_timestamp("2026-02-06").unwrap(), "2026-02-06");
        assert_eq!(
            parse_date_or_timestamp("2026-04-12 09:35:16").unwrap(),
            "2026-04-12 09:35:16"
        );
        assert_eq!(
            parse_date_or_timestamp("2026-04-12T13:35:16Z").unwrap(),
            "2026-04-12 09:35:16"
        );
    }

    #[test]
    fn compare_date_or_timestamp_does_not_invent_intraday_order_for_date_only_values() {
        assert_eq!(
            compare_date_or_timestamp("2026-02-06", "2026-02-07").unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_date_or_timestamp("2026-02-07 09:30:00", "2026-02-07 15:45:00").unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_date_or_timestamp("2026-02-07", "2026-02-07 15:45:00").unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            compare_date_or_timestamp("2026-02-07 15:45:00", "2026-02-07").unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn first_date_or_timestamp_picks_first_non_empty_canonical_value() {
        assert_eq!(
            first_date_or_timestamp([None, Some(""), Some("2026-02-07")]),
            Some("2026-02-07".to_string())
        );
        assert_eq!(
            first_date_or_timestamp([None, Some("2026-04-12T13:35:16Z")]),
            Some("2026-04-12 09:35:16".to_string())
        );
        assert_eq!(first_date_or_timestamp([None, Some(""), None]), None);
    }

    #[test]
    fn short_clock_names_return_canonical_values() {
        let current = now();
        assert_eq!(parse_timestamp(&current).unwrap(), current);
        assert_eq!(today(), current[0..10].to_string());
    }

    #[test]
    fn parts_extract_canonical_timestamp_fields() {
        let value = parts(Some("2026-04-12T02:35:16Z")).unwrap();
        assert_eq!(value.timestamp, "2026-04-11 22:35:16");
        assert_eq!(value.date, "2026-04-11");
        assert_eq!(value.hour, 22);
        assert_eq!(value.minute, 35);
        assert_eq!(value.second, 16);
        assert_eq!(value.hhmm, 2235);
        assert_eq!(value.hhmm_string, "22:35");
    }

    #[test]
    fn rust_adapters_absorb_canonical_and_rfc3339_inputs() {
        assert_eq!(
            chrono::date(Some("2026-04-12T02:35:16Z")).unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 11).unwrap()
        );
        assert_eq!(
            chrono::timestamp(Some("2026-04-12T02:35:16Z")).unwrap(),
            NaiveDateTime::parse_from_str("2026-04-11 22:35:16", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }
}
