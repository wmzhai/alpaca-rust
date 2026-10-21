use alpaca_option::numeric::bivariate_normal_cdf;
use alpaca_option::pricing::{greeks_default, price_black_scholes, price_default};
use alpaca_option::probability::expiry_probability_in_range_with_rate;
use alpaca_option::{BlackScholesInput, OptionRight};

fn sample(option_right: OptionRight, dividend_yield: f64) -> BlackScholesInput {
    BlackScholesInput {
        spot: 100.0,
        strike: 100.0,
        years: 0.75,
        rate: 0.05,
        dividend_yield,
        volatility: 0.25,
        option_right,
    }
}

#[test]
fn zero_dividend_call_matches_european() {
    let input = sample(OptionRight::Call, 0.0);
    let american = price_default(&input).expect("american call");
    let european = price_black_scholes(&input).expect("european call");
    assert!((american - european).abs() < 1e-8, "{american} vs {european}");
}

#[test]
fn zero_dividend_put_is_above_european() {
    let input = sample(OptionRight::Put, 0.0);
    let american = price_default(&input).expect("american put");
    let european = price_black_scholes(&input).expect("european put");
    assert!(american + 1e-8 >= european, "{american} vs {european}");
    assert!((american - 7.103_512).abs() < 1e-4, "{american}");
}

#[test]
fn spot_zero_is_the_american_intrinsic_value() {
    let mut input = sample(OptionRight::Call, 0.0);
    input.spot = 0.0;
    assert_eq!(price_default(&input).expect("call at zero"), 0.0);
    input.option_right = OptionRight::Put;
    assert_eq!(price_default(&input).expect("put at zero"), input.strike);
    input.spot = -1.0;
    assert!(price_default(&input).is_err());
    input.spot = 0.0;
    input.option_right = OptionRight::Call;
    assert!(price_black_scholes(&input).is_err());
}

#[test]
fn several_expirations_scan_five_moves_and_price_zero() {
    use alpaca_option::probability::profit_loss_bounds;
    use std::cell::RefCell;

    let seen = RefCell::new(Vec::new());
    let bounds = profit_loss_bounds(
        |price| {
            seen.borrow_mut().push(price);
            if price == 0.0 {
                -4153.0
            } else {
                -268.0
            }
        },
        160.0,
        &[115.0, 150.0],
        false,
        80.0,
        20,
    )
    .expect("bounds");
    assert!((bounds.min_pnl - -4153.0).abs() < 1e-9, "{}", bounds.min_pnl);
    let prices = seen.borrow();
    assert!(prices.iter().any(|price| *price == 0.0));
    assert!(prices.iter().any(|price| *price >= 160.0 * 100.0));
    let grid: Vec<f64> = prices
        .iter()
        .copied()
        .filter(|price| *price > 0.0 && *price < 1000.0)
        .collect();
    assert!(
        grid.iter().all(|price| *price <= 560.0 + 1e-6),
        "{grid:?}"
    );
}

#[test]
fn default_greeks_are_finite() {
    let greeks = greeks_default(&sample(OptionRight::Put, 0.0)).expect("greeks");
    assert!(greeks.delta.is_finite());
    assert!(greeks.gamma.is_finite());
    assert!(greeks.theta.is_finite());
    assert!(greeks.vega.is_finite());
    assert!(greeks.rho.is_finite());
}

#[test]
fn bivariate_orthant_matches_closed_form() {
    let actual = bivariate_normal_cdf(0.0, 0.0, 0.5);
    let expected = 0.25 + (0.5_f64.asin()) / (2.0 * std::f64::consts::PI);
    assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
}

#[test]
fn probability_ignores_rate_and_dividend() {
    let base = expiry_probability_in_range_with_rate(100.0, 80.0, 120.0, 0.5, 0.01, 0.0, 0.2)
        .expect("base");
    let shifted =
        expiry_probability_in_range_with_rate(100.0, 80.0, 120.0, 0.5, 0.08, 0.03, 0.2)
            .expect("shifted");
    assert!((base - shifted).abs() < 1e-12);
}
