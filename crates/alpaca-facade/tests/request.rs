use alpaca_facade::OptionChainRequest;
use alpaca_option::OptionRight;
use rust_decimal::Decimal;

#[test]
fn request_from_dte_range_builds_ny_date_window_and_rounded_strikes() {
    let request = OptionChainRequest::from_dte_range(7, 21, Some(432.126), Some(441.874));

    assert_eq!(request.strike_price_gte(), Some(Decimal::new(43213, 2)));
    assert_eq!(request.strike_price_lte(), Some(Decimal::new(44187, 2)));
    assert!(
        request
            .expiration_date_gte()
            .is_some_and(|value| value.len() == 10)
    );
    assert!(
        request
            .expiration_date_lte()
            .is_some_and(|value| value.len() == 10)
    );
}

#[test]
fn request_from_expiration_range_keeps_exact_dates() {
    let request = OptionChainRequest::from_expiration_range(Some("2026-05-15"), Some("2026-06-19"));

    assert_eq!(request.expiration_date_gte(), Some("2026-05-15"));
    assert_eq!(request.expiration_date_lte(), Some("2026-06-19"));
}

#[test]
fn request_from_expiration_date_sets_exact_filter_only() {
    let request = OptionChainRequest::from_expiration_date("2026-07-22");

    assert_eq!(request.expiration_date(), Some("2026-07-22"));
    assert_eq!(request.expiration_date_gte(), None);
    assert_eq!(request.expiration_date_lte(), None);
}

#[test]
fn request_with_strike_range_rounds_float_inputs() {
    let request = OptionChainRequest::new().with_strike_range(Some(430.126), Some(441.874));

    assert_eq!(request.strike_price_gte(), Some(Decimal::new(43013, 2)));
    assert_eq!(request.strike_price_lte(), Some(Decimal::new(44187, 2)));
}

#[test]
fn request_with_option_type_sets_right() {
    let request = OptionChainRequest::new().with_option_type(OptionRight::Put);

    assert_eq!(request.option_type(), Some(&OptionRight::Put));
}

#[test]
fn request_with_underlying_price_only_fills_valid_positive_values() {
    let request = OptionChainRequest::new().with_underlying_price(Some(512.25));
    assert_eq!(request.underlying_price(), Some(512.25));

    let unchanged = request.clone().with_underlying_price(Some(0.0));
    assert_eq!(unchanged.underlying_price(), Some(512.25));
}

#[test]
fn request_covers_narrower_window_and_same_option_type() {
    let cached = OptionChainRequest::from_expiration_range(Some("2026-04-20"), Some("2026-05-16"))
        .with_option_type(OptionRight::Call)
        .with_strike_range(Some(90.0), Some(110.0))
        .with_underlying_price(Some(100.0));
    let requested =
        OptionChainRequest::from_expiration_range(Some("2026-04-24"), Some("2026-05-09"))
            .with_option_type(OptionRight::Call)
            .with_strike_range(Some(95.0), Some(105.0))
            .with_underlying_price(Some(101.5));

    assert!(cached.covers(&requested));
    assert!(!requested.covers(&cached));
}

#[test]
fn request_covers_exact_date_with_range() {
    let cached = OptionChainRequest::from_expiration_range(Some("2026-07-01"), Some("2026-08-31"))
        .with_option_type(OptionRight::Call);
    let requested = OptionChainRequest::from_expiration_date("2026-07-15").with_option_type(OptionRight::Call);

    assert!(cached.covers(&requested));
    assert!(matches!(requested.expiration_date(), Some("2026-07-15")));
}

#[test]
fn request_does_not_cover_different_exact_date() {
    let cached = OptionChainRequest::from_expiration_date("2026-07-22");
    let requested = OptionChainRequest::from_expiration_date("2026-07-23");

    assert!(!cached.covers(&requested));
    assert!(!requested.covers(&cached));
}

#[test]
fn request_merge_expands_bounds_and_promotes_option_type_to_all_when_needed() {
    let mut merged =
        OptionChainRequest::from_expiration_range(Some("2026-04-24"), Some("2026-05-09"))
            .with_option_type(OptionRight::Call)
            .with_strike_range(Some(95.0), Some(105.0))
            .with_underlying_price(Some(101.5));

    merged.merge(
        &OptionChainRequest::from_expiration_range(Some("2026-04-20"), Some("2026-05-16"))
            .with_option_type(OptionRight::Put)
            .with_strike_range(Some(90.0), Some(110.0))
            .with_underlying_price(Some(99.8)),
    );

    assert_eq!(merged.option_type(), None);
    assert_eq!(merged.strike_price_gte(), Some(Decimal::new(9000, 2)));
    assert_eq!(merged.strike_price_lte(), Some(Decimal::new(11000, 2)));
    assert_eq!(merged.expiration_date_gte(), Some("2026-04-20"));
    assert_eq!(merged.expiration_date_lte(), Some("2026-05-16"));
    assert_eq!(merged.underlying_price(), Some(101.5));
}

#[test]
fn request_merge_exact_with_range_expands_to_range() {
    let mut merged =
        OptionChainRequest::from_expiration_date("2026-07-22").with_option_type(OptionRight::Call);
    merged.merge(
        &OptionChainRequest::from_expiration_range(Some("2026-07-24"), Some("2026-07-25"))
            .with_option_type(OptionRight::Call),
    );

    assert_eq!(merged.expiration_date(), None);
    assert_eq!(merged.expiration_date_gte(), None);
    assert_eq!(merged.expiration_date_lte(), None);
}
