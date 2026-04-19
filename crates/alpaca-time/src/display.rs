use chrono::Datelike;

use crate::clock::{minutes_from_hhmm, parse_naive_date, parse_naive_timestamp, parse_timestamp};
use crate::error::{TimeError, TimeResult};
use crate::types::{DurationParts, WeekdayCode};

fn compact_date(date: &str, style: &str) -> TimeResult<String> {
    let date = parse_naive_date(date)?;
    match style {
        "mm-dd" => Ok(date.format("%m-%d").to_string()),
        "yy-mm-dd" => Ok(date.format("%y-%m-%d").to_string()),
        "yymmdd" => Ok(date.format("%y%m%d").to_string()),
        _ => Err(TimeError::new(
            "invalid_compact_date_style",
            format!("invalid compact date style: {style}"),
        )),
    }
}

fn compact_timestamp(timestamp: &str, style: &str) -> TimeResult<String> {
    let timestamp = parse_naive_timestamp(&parse_timestamp(timestamp)?)?;
    match style {
        "mm-dd hh:mm" => Ok(timestamp.format("%m-%d %H:%M").to_string()),
        "yy-mm-dd hh:mm" => Ok(timestamp.format("%y-%m-%d %H:%M").to_string()),
        "yyyy-mm-dd hh:mm" => Ok(timestamp.format("%Y-%m-%d %H:%M").to_string()),
        _ => Err(TimeError::new(
            "invalid_compact_timestamp_style",
            format!("invalid compact timestamp style: {style}"),
        )),
    }
}

fn time_only(timestamp: &str, precision: &str) -> TimeResult<String> {
    let timestamp = parse_naive_timestamp(&parse_timestamp(timestamp)?)?;
    match precision {
        "minute" => Ok(timestamp.format("%H:%M").to_string()),
        "second" => Ok(timestamp.format("%H:%M:%S").to_string()),
        other => Err(TimeError::new(
            "invalid_time_only_precision",
            format!("invalid time precision: {other}"),
        )),
    }
}

pub fn compact(input: &str, style: &str) -> String {
    let compacted = if style.contains("hh:mm") {
        if input.len() == 10 {
            match style {
                "mm-dd hh:mm" => compact_date(input, "mm-dd"),
                "yy-mm-dd hh:mm" => compact_date(input, "yy-mm-dd"),
                "yyyy-mm-dd hh:mm" => crate::clock::parse_date(input),
                _ => compact_timestamp(input, style),
            }
        } else {
            compact_timestamp(input, style)
        }
    } else if input.len() == 10 {
        compact_date(input, style)
    } else {
        parse_timestamp(input).and_then(|timestamp| compact_date(&timestamp[0..10], style))
    };

    compacted.unwrap_or_else(|_| input.to_string())
}

pub fn time(input: &str, precision: Option<&str>, date_style: Option<&str>) -> String {
    if input.len() == 10 {
        return compact_date(input, date_style.unwrap_or("mm-dd"))
            .unwrap_or_else(|_| input.to_string());
    }

    time_only(input, precision.unwrap_or("minute")).unwrap_or_else(|_| input.to_string())
}

pub fn hhmm(input: &str) -> TimeResult<String> {
    if input.contains(':') {
        minutes_from_hhmm(input)?;
        return Ok(input.to_string());
    }

    if input.len() != 4 || !input.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(TimeError::new(
            "invalid_hhmm_compact",
            format!("invalid compact hhmm: {input}"),
        ));
    }

    let hour = input[0..2].parse::<u32>().map_err(|_| {
        TimeError::new(
            "invalid_hhmm_compact",
            format!("invalid compact hhmm: {input}"),
        )
    })?;
    let minute = input[2..4].parse::<u32>().map_err(|_| {
        TimeError::new(
            "invalid_hhmm_compact",
            format!("invalid compact hhmm: {input}"),
        )
    })?;

    crate::clock::hhmm_string_from_parts(hour, minute)
}

pub fn duration(start: &str, end: &str) -> String {
    let Ok(start_minutes) = minutes_from_hhmm(start) else {
        return "-".to_string();
    };
    let Ok(end_minutes) = minutes_from_hhmm(end) else {
        return "-".to_string();
    };

    let diff_minutes = end_minutes.saturating_sub(start_minutes);
    let hours = diff_minutes / 60;
    let minutes = diff_minutes % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

pub fn weekday_code(date: &str) -> TimeResult<WeekdayCode> {
    let date = parse_naive_date(date)?;
    Ok(match date.weekday() {
        chrono::Weekday::Mon => WeekdayCode::Mon,
        chrono::Weekday::Tue => WeekdayCode::Tue,
        chrono::Weekday::Wed => WeekdayCode::Wed,
        chrono::Weekday::Thu => WeekdayCode::Thu,
        chrono::Weekday::Fri => WeekdayCode::Fri,
        chrono::Weekday::Sat => WeekdayCode::Sat,
        chrono::Weekday::Sun => WeekdayCode::Sun,
    })
}

pub fn duration_parts(seconds: i64) -> DurationParts {
    let sign = if seconds > 0 {
        1
    } else if seconds < 0 {
        -1
    } else {
        0
    };
    let total_seconds = seconds.abs();
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    DurationParts {
        sign,
        total_seconds,
        days,
        hours,
        minutes,
        seconds,
    }
}

pub fn relative_duration_parts(from: &str, to: &str) -> TimeResult<DurationParts> {
    let from = parse_naive_timestamp(&parse_timestamp(from)?)?;
    let to = parse_naive_timestamp(&parse_timestamp(to)?)?;
    Ok(duration_parts(to.signed_duration_since(from).num_seconds()))
}

pub fn compact_duration(parts: &DurationParts, style: Option<&str>) -> TimeResult<String> {
    let sign = if parts.sign < 0 { "-" } else { "" };
    let style = style.unwrap_or("hm");

    let body = match style {
        "hm" => {
            let total_hours = parts.days * 24 + parts.hours;
            if total_hours > 0 {
                format!("{total_hours}h {}m", parts.minutes)
            } else {
                format!("{}m", parts.minutes)
            }
        }
        "dhm" => {
            if parts.days > 0 {
                format!("{}d {}h {}m", parts.days, parts.hours, parts.minutes)
            } else if parts.hours > 0 {
                format!("{}h {}m", parts.hours, parts.minutes)
            } else {
                format!("{}m", parts.minutes)
            }
        }
        other => {
            return Err(TimeError::new(
                "invalid_compact_duration_style",
                format!("invalid compact duration style: {other}"),
            ));
        }
    };

    Ok(format!("{sign}{body}"))
}

pub fn compact_days_until(days: f64) -> String {
    if days.abs() < 1.0 {
        format!("{:.1}h", days * 24.0)
    } else {
        format!("{days:.1}D")
    }
}
