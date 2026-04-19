use chrono::{NaiveDate, NaiveDateTime};

use crate::TimeResult;
use crate::clock::{now, parse_naive_date, parse_naive_timestamp, today};

pub fn date(input: Option<&str>) -> TimeResult<NaiveDate> {
    match input {
        Some(value) => {
            if let Ok(timestamp) = parse_naive_timestamp(value) {
                return Ok(timestamp.date());
            }
            parse_naive_date(value)
        }
        None => parse_naive_date(&today()),
    }
}

pub fn timestamp(input: Option<&str>) -> TimeResult<NaiveDateTime> {
    parse_naive_timestamp(input.unwrap_or(&now()))
}
