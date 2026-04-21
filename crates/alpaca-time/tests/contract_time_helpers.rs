use alpaca_time::clock;

#[test]
fn parses_compact_and_colon_hhmm_into_single_canonical_value() {
    assert_eq!(clock::parse_hhmm("0931").unwrap(), "09:31");
    assert_eq!(clock::parse_hhmm("09:31").unwrap(), "09:31");
    assert_eq!(clock::compact_hhmm("09:31").unwrap(), "0931");
    assert_eq!(clock::compact_hhmm("0931").unwrap(), "0931");
}

#[test]
fn builds_minute_timestamp_from_date_and_hhmm() {
    assert_eq!(
        clock::minute_timestamp("2026-04-20", "0931").unwrap(),
        "2026-04-20 09:31:00"
    );
    assert_eq!(
        clock::minute_timestamp("2026-04-20", "09:31").unwrap(),
        "2026-04-20 09:31:00"
    );
}
