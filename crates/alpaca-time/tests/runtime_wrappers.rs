use alpaca_time::{calendar, clock, expiration, range, session};

#[test]
fn fractional_day_wrappers_track_current_time() {
    let now = clock::now();
    let tolerance = 2.0 / 86_400.0;

    let until = clock::fractional_days_until(&now).expect("fractional_days_until should work");
    let since = clock::fractional_days_since(&now).expect("fractional_days_since should work");

    assert!(
        until.abs() <= tolerance,
        "until={until} exceeded tolerance={tolerance}"
    );
    assert!(
        since.abs() <= tolerance,
        "since={since} exceeded tolerance={tolerance}"
    );
}

#[test]
fn clock_helpers_absorb_common_fallbacks() {
    assert_eq!(
        clock::truncate_to_minute("2025-02-06 11:30"),
        "2025-02-06 11:30:00"
    );
    assert_eq!(clock::from_unix_seconds(0), "");
}

#[test]
fn calendar_and_range_helpers_absorb_common_fallbacks() {
    let today = clock::today();
    assert_eq!(range::add_days(&today, 1).unwrap().len(), 10);
    assert!(!range::is_last_trading_date_of_week("bad-date"));
    assert!(!calendar::is_trading_date("bad-date"));
    assert_eq!(
        calendar::is_trading_today(),
        calendar::is_trading_date(&today)
    );
}

#[test]
fn session_bool_helpers_absorb_invalid_input() {
    assert_eq!(
        session::is_regular_session_now(),
        session::is_regular_session_at(&clock::now())
    );
    assert_eq!(
        session::is_overnight_now(),
        session::is_overnight_window(&clock::now())
    );
    assert_eq!(
        session::is_in_window_now("09:30", "16:00"),
        session::is_in_window(&clock::now(), "09:30", "16:00")
    );
    assert!(!session::is_regular_session_at("bad-timestamp"));
    assert!(!session::is_in_window("bad-timestamp", "09:30", "16:00"));
    assert!(!session::is_overnight_window("bad-timestamp"));
}

#[test]
fn session_and_calendar_helpers_accept_rfc3339_utc() {
    assert!(session::is_regular_session_at("2025-02-06T15:00:00Z"));
    assert!(!session::is_overnight_window("2025-02-06T15:00:00Z"));
    assert_eq!(
        calendar::last_completed_trading_date(Some("2025-02-06T22:00:00Z")).unwrap(),
        "2025-02-06"
    );
}

#[test]
fn years_absorb_elapsed_and_invalid_input() {
    assert_eq!(
        expiration::years("2025-02-06", Some("2025-02-06 16:00:01"), None),
        0.0
    );
    assert_eq!(
        expiration::years("2025-02-06", Some("bad-timestamp"), None),
        0.0
    );
}
