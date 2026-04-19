use alpaca_option::numeric;

#[test]
fn evaluate_points_returns_function_values_in_order() {
    let values = numeric::evaluate_points(&[1.0, 2.0, 3.5], |spot| Ok(spot * spot)).unwrap();
    assert_eq!(values, vec![1.0, 4.0, 12.25]);
}

#[test]
fn refine_bracketed_root_solves_sign_change_interval() {
    let root = numeric::refine_bracketed_root(1.0, 2.0, |spot| Ok(spot * spot - 2.0), Some(1e-9), Some(100))
        .unwrap();
    assert!((root - 2.0_f64.sqrt()).abs() < 1e-7, "root={root}");
}

#[test]
fn scan_range_extrema_finds_min_and_max_points() {
    let extrema = numeric::scan_range_extrema(90.0, 110.0, Some(5.0), |spot| Ok((spot - 100.0) * 2.0))
        .unwrap();

    assert_eq!(extrema.min_spot, 90.0);
    assert_eq!(extrema.min_value, -20.0);
    assert_eq!(extrema.max_spot, 110.0);
    assert_eq!(extrema.max_value, 20.0);
}
