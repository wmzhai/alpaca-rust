use alpaca_time::DurationParts;
use alpaca_time::{calendar, clock, display, expiration, range, session};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_cases(relative_path: &str) -> Vec<Value> {
    let content =
        fs::read_to_string(repo_root().join(relative_path)).expect("fixture should exist");
    serde_json::from_str::<Value>(&content)
        .expect("fixture json should parse")
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .expect("fixture cases should be array")
}

fn unwrap_expected(expected: &Value) -> Value {
    expected
        .get("value")
        .cloned()
        .unwrap_or_else(|| expected.clone())
}

fn parse_duration_parts(value: &Value) -> DurationParts {
    DurationParts {
        sign: value.get("sign").and_then(Value::as_i64).unwrap() as i8,
        total_seconds: value.get("total_seconds").and_then(Value::as_i64).unwrap(),
        days: value.get("days").and_then(Value::as_i64).unwrap(),
        hours: value.get("hours").and_then(Value::as_i64).unwrap(),
        minutes: value.get("minutes").and_then(Value::as_i64).unwrap(),
        seconds: value.get("seconds").and_then(Value::as_i64).unwrap(),
    }
}

fn assert_with_tolerance(actual: Value, expected: Value, tolerance: Option<f64>) {
    match (actual, expected, tolerance) {
        (Value::Number(a), Value::Number(e), Some(tol)) => {
            let a = a.as_f64().unwrap();
            let e = e.as_f64().unwrap();
            assert!((a - e).abs() <= tol, "expected {a} ~= {e} with tol {tol}");
        }
        (Value::Number(a), Value::Number(e), None) => {
            assert_eq!(a.as_f64().unwrap(), e.as_f64().unwrap());
        }
        (actual, expected, _) => assert_eq!(actual, expected),
    }
}

fn run_case(case: &Value) -> Value {
    let api = case.get("api").and_then(Value::as_str).unwrap();
    let input = case.get("input").unwrap();

    match api {
        "clock.parse_date" => serde_json::to_value(
            clock::parse_date(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "clock.parse_timestamp" => serde_json::to_value(
            clock::parse_timestamp(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "clock.to_utc_rfc3339" => serde_json::to_value(
            clock::to_utc_rfc3339(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "clock.from_unix_seconds" => serde_json::to_value(clock::from_unix_seconds(
            input.get("seconds").unwrap().as_i64().unwrap(),
        ))
        .unwrap(),
        "clock.truncate_to_minute" => serde_json::to_value(clock::truncate_to_minute(
            input.get("input").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "clock.minute_key" => serde_json::to_value(
            clock::minute_key(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "clock.hhmm_string_from_parts" => serde_json::to_value(
            clock::hhmm_string_from_parts(
                input.get("hour").unwrap().as_i64().unwrap() as u32,
                input.get("minute").unwrap().as_i64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "clock.minutes_from_hhmm" => serde_json::to_value(
            clock::minutes_from_hhmm(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "clock.fractional_days_between" => serde_json::to_value(
            clock::fractional_days_between(
                input.get("start").unwrap().as_str().unwrap(),
                input.get("end").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "calendar.trading_day_info" => serde_json::to_value(
            calendar::trading_day_info(input.get("date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "calendar.is_trading_date" => serde_json::to_value(calendar::is_trading_date(
            input.get("date").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "calendar.market_hours_for_date" => serde_json::to_value(
            calendar::market_hours_for_date(input.get("date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "calendar.last_completed_trading_date" => serde_json::to_value(
            calendar::last_completed_trading_date(input.get("timestamp").and_then(Value::as_str))
                .unwrap(),
        )
        .unwrap(),
        "calendar.add_trading_days" => serde_json::to_value(
            calendar::add_trading_days(
                input.get("date").unwrap().as_str().unwrap(),
                input.get("days").unwrap().as_i64().unwrap() as i32,
            )
            .unwrap(),
        )
        .unwrap(),
        "session.market_session_at" => serde_json::to_value(
            session::market_session_at(input.get("timestamp").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "session.is_overnight_window" => serde_json::to_value(session::is_overnight_window(
            input.get("timestamp").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "expiration.close" => serde_json::to_value(
            expiration::close(input.get("expiration_date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "expiration.calendar_days" => serde_json::to_value(
            expiration::calendar_days(
                input.get("expiration_date").unwrap().as_str().unwrap(),
                input.get("timestamp").and_then(Value::as_str),
            )
            .unwrap(),
        )
        .unwrap(),
        "expiration.days" => serde_json::to_value(
            expiration::days(
                input.get("expiration_date").unwrap().as_str().unwrap(),
                input.get("timestamp").and_then(Value::as_str),
            )
            .unwrap(),
        )
        .unwrap(),
        "expiration.years" => serde_json::to_value(expiration::years(
            input.get("expiration_date").unwrap().as_str().unwrap(),
            input.get("timestamp").and_then(Value::as_str),
            input.get("basis").and_then(Value::as_str),
        ))
        .unwrap(),
        "expiration.years_between_dates" => serde_json::to_value(
            expiration::years_between_dates(
                input.get("start_date").unwrap().as_str().unwrap(),
                input.get("end_date").unwrap().as_str().unwrap(),
                input.get("basis").and_then(Value::as_str),
            )
            .unwrap(),
        )
        .unwrap(),
        "range.add_days" => serde_json::to_value(
            range::add_days(
                input.get("date").unwrap().as_str().unwrap(),
                input.get("days").unwrap().as_i64().unwrap() as i32,
            )
            .unwrap(),
        )
        .unwrap(),
        "range.dates" => serde_json::to_value(
            range::dates(
                input.get("start_date").unwrap().as_str().unwrap(),
                input.get("end_date").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "range.trading_dates" => serde_json::to_value(
            range::trading_dates(
                input.get("start_date").unwrap().as_str().unwrap(),
                input.get("end_date").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "range.nth_weekday" => serde_json::to_value(
            range::nth_weekday(
                input.get("year").unwrap().as_i64().unwrap() as i32,
                input.get("month").unwrap().as_u64().unwrap() as u32,
                input.get("weekday").unwrap().as_str().unwrap(),
                input.get("nth").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "range.is_last_trading_date_of_week" => serde_json::to_value(
            range::is_last_trading_date_of_week(input.get("date").unwrap().as_str().unwrap()),
        )
        .unwrap(),
        "range.weekly_last_trading_dates" => serde_json::to_value(
            range::weekly_last_trading_dates(
                input.get("start_date").unwrap().as_str().unwrap(),
                input.get("end_date").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "range.last_trading_date_of_week" => serde_json::to_value(
            range::last_trading_date_of_week(input.get("date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "range.calendar_week_range" => serde_json::to_value(
            range::calendar_week_range(input.get("date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "range.iso_week_range" => serde_json::to_value(
            range::iso_week_range(
                input.get("year").unwrap().as_i64().unwrap() as i32,
                input.get("week").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "display.compact" => serde_json::to_value(display::compact(
            input
                .get("date")
                .or_else(|| input.get("timestamp"))
                .unwrap()
                .as_str()
                .unwrap(),
            input.get("style").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "display.time" => serde_json::to_value(display::time(
            input
                .get("date")
                .or_else(|| input.get("timestamp"))
                .unwrap()
                .as_str()
                .unwrap(),
            input.get("precision").and_then(Value::as_str),
            None,
        ))
        .unwrap(),
        "display.hhmm" => serde_json::to_value(
            display::hhmm(input.get("input").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "display.duration" => serde_json::to_value(display::duration(
            input.get("start").unwrap().as_str().unwrap(),
            input.get("end").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "display.weekday_code" => serde_json::to_value(
            display::weekday_code(input.get("date").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "display.relative_duration_parts" => serde_json::to_value(
            display::relative_duration_parts(
                input.get("from").unwrap().as_str().unwrap(),
                input.get("to").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "display.compact_duration" => serde_json::to_value(
            display::compact_duration(
                &parse_duration_parts(input.get("parts").unwrap()),
                input.get("style").and_then(Value::as_str),
            )
            .unwrap(),
        )
        .unwrap(),
        "display.compact_days_until" => serde_json::to_value(display::compact_days_until(
            input.get("days").unwrap().as_f64().unwrap(),
        ))
        .unwrap(),
        other => panic!("Unhandled fixture api: {other}"),
    }
}

#[test]
fn fixture_suite() {
    for fixture_path in [
        "fixtures/parsing/clock-basics.json",
        "fixtures/calendar/trading-days.json",
        "fixtures/session/market-sessions.json",
        "fixtures/expiration/expiration-math.json",
        "fixtures/range/date-ranges.json",
        "fixtures/dst/dst-boundaries.json",
        "fixtures/display/display-formats.json",
    ] {
        for case in load_cases(fixture_path) {
            let actual = run_case(&case);
            let expected = unwrap_expected(case.get("expected").unwrap());
            let tolerance = case.get("tolerance").and_then(Value::as_f64);
            assert_with_tolerance(actual, expected, tolerance);
        }
    }
}
