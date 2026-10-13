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
