use alpaca_time::{clock, expiration};

#[test]
fn years_clamp_elapsed_expiration() {
    assert_eq!(
        expiration::years("2025-02-06", Some("2025-02-06 16:00:01"), None),
        0.0
    );
}

#[test]
fn now_round_trips_to_canonical_timestamp() {
    let now = clock::now();
    assert_eq!(clock::parse_timestamp(&now).unwrap(), now);
}
