use alpaca_time::{clock, display, expiration, range};

#[test]
fn canonical_clock_api_uses_short_names() {
    let now = clock::now();
    assert_eq!(clock::parse_timestamp(&now).unwrap(), now);
    assert_eq!(clock::today(), now[0..10].to_string());
    let parts = clock::parts(Some("2026-04-12T02:35:16Z")).unwrap();
    assert_eq!(parts.date, "2026-04-11");
    assert_eq!(parts.timestamp, "2026-04-11 22:35:16");
    assert_eq!(parts.hour, 22);
    assert_eq!(parts.minute, 35);
    assert_eq!(parts.second, 16);
    assert_eq!(parts.hhmm, 2235);
    assert_eq!(parts.hhmm_string, "22:35");
    assert_eq!(parts.weekday_from_sunday, 6);
}

#[test]
fn canonical_display_api_absorbs_date_timestamp_and_invalid_inputs() {
    assert_eq!(display::compact("2026-02-06", "mm-dd hh:mm"), "02-06");
    assert_eq!(display::compact("2026-04-12T02:35:16Z", "mm-dd hh:mm"), "04-11 22:35");
    assert_eq!(display::compact("2026-04-12 09:35:16", "mm-dd"), "04-12");
    assert_eq!(display::compact("bad-timestamp", "mm-dd hh:mm"), "bad-timestamp");

    assert_eq!(display::time("2026-02-06", Some("minute"), Some("mm-dd")), "02-06");
    assert_eq!(display::time("2026-04-12T02:35:16Z", Some("second"), Some("mm-dd")), "22:35:16");
    assert_eq!(display::time("bad-timestamp", Some("minute"), Some("mm-dd")), "bad-timestamp");
}

#[test]
fn canonical_expiration_api_makes_at_optional_and_clamps_years() {
    assert_eq!(expiration::close("2025-02-06").unwrap(), "2025-02-06 16:00:00");
    assert_eq!(expiration::calendar_days("2025-02-06", Some("2025-02-06 15:59:59")).unwrap(), 0);
    assert_eq!(expiration::calendar_days("2025-02-06", Some("2025-02-06 16:00:01")).unwrap(), -1);
    assert_eq!(expiration::days("2025-02-07", Some("2025-02-06 16:00:00")).unwrap(), 1.0);
    assert_eq!(expiration::years("2025-02-06", Some("2025-02-06 16:00:01"), None), 0.0);
    assert_eq!(expiration::years("2025-02-06", Some("bad-timestamp"), None), 0.0);
}

#[test]
fn canonical_range_api_removes_date_wrappers_and_exposes_generic_weekday_selection() {
    assert_eq!(range::add_days("2025-02-06", 7).unwrap(), "2025-02-13");
    assert_eq!(
        range::dates("2025-02-06", "2025-02-08").unwrap(),
        vec!["2025-02-06", "2025-02-07", "2025-02-08"],
    );
    assert_eq!(
        range::trading_dates("2025-02-06", "2025-02-10").unwrap(),
        vec!["2025-02-06", "2025-02-07", "2025-02-10"],
    );
    assert_eq!(range::nth_weekday(2025, 3, "fri", 3).unwrap(), "2025-03-21");
}
