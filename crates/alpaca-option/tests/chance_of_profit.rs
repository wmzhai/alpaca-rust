use alpaca_option::margin::{strategy_collateral, CollateralLeg};
use alpaca_option::probability::{
    chance_of_profit, chance_of_target, directed_break_evens, polished_directed_break_evens,
    profit_loss_bounds, ChanceOfProfitInput, DirectedBreakEven,
};

const SPOT: f64 = 100.0;
const DAYS: f64 = 365.0;
const VOLATILITY: f64 = 0.2;

fn island(low: f64, high: f64) -> f64 {
    let lower = chance_of_target(SPOT, low, DAYS, VOLATILITY)
        .expect("lower")
        .below;
    let upper = chance_of_target(SPOT, high, DAYS, VOLATILITY)
        .expect("upper")
        .below;
    upper - lower
}

fn profit(breakevens: &[DirectedBreakEven], max_profit: f64, min_pnl: f64) -> f64 {
    chance_of_profit(ChanceOfProfitInput {
        spot: SPOT,
        days: DAYS,
        volatility: VOLATILITY,
        breakevens,
        max_profit,
        min_pnl,
        pnl_without_break_even: -1.0,
    })
    .expect("chance")
}

#[test]
fn one_breakeven_uses_its_direction() {
    let above = profit(&[DirectedBreakEven { at: 105.0, direction: 1 }], 100.0, -50.0);
    let tail = chance_of_target(SPOT, 105.0, DAYS, VOLATILITY)
        .expect("target")
        .above;
    assert!((above - tail).abs() < 1e-12, "{above} vs {tail}");

    let under = profit(
        &[DirectedBreakEven {
            at: 105.0,
            direction: -1,
        }],
        100.0,
        -50.0,
    );
    assert!((under - (1.0 - tail)).abs() < 1e-12, "{under}");
}

#[test]
fn two_profit_islands_add_both_masses() {
    let breakevens = [
        DirectedBreakEven { at: 80.0, direction: 1 },
        DirectedBreakEven { at: 90.0, direction: -1 },
        DirectedBreakEven { at: 110.0, direction: 1 },
        DirectedBreakEven { at: 130.0, direction: -1 },
    ];
    let actual = profit(&breakevens, f64::INFINITY, f64::NEG_INFINITY);
    let expected = island(80.0, 90.0) + island(110.0, 130.0);
    assert!((actual - expected).abs() < 1e-12, "{actual} vs {expected}");
}

#[test]
fn impossible_profit_is_zero_and_impossible_loss_is_one() {
    let breakevens = [
        DirectedBreakEven { at: 90.0, direction: 1 },
        DirectedBreakEven {
            at: 110.0,
            direction: -1,
        },
    ];
    assert_eq!(profit(&breakevens, 0.0, -10.0), 0.0);
    assert_eq!(profit(&breakevens, 10.0, 0.0), 1.0);
}

#[test]
fn sampled_sign_changes_keep_every_direction() {
    let roots = directed_break_evens(&[70.0, 85.0, 100.0, 115.0, 140.0], &[-5.0, 5.0, -5.0, 5.0, -5.0])
        .expect("roots");
    let directions: Vec<i8> = roots.iter().map(|root| root.direction).collect();
    let places: Vec<f64> = roots.iter().map(|root| root.at).collect();
    assert_eq!(directions, vec![1, -1, 1, -1]);
    assert_eq!(places, vec![77.5, 92.5, 107.5, 127.5]);
}

#[test]
fn single_expiration_wing_that_keeps_rising_is_unlimited() {
    let bounds = profit_loss_bounds(|price| price, 100.0, &[100.0], true, 10.0, 20).expect("bounds");
    assert!(bounds.max_profit.is_infinite());
    assert!(bounds.min_pnl.is_finite());
}

#[test]
fn cash_secured_put_collateral_is_strike_times_size() {
    let amount = strategy_collateral(
        "cash-secured-put",
        &[CollateralLeg {
            long: false,
            call: false,
            strike: 50.0,
            basis: 1.2,
            size: 2.0,
        }],
        55.0,
    );
    assert_eq!(amount, Some(10_000.0));
}

#[test]
fn polished_root_lands_on_a_linear_zero() {
    let roots = polished_directed_break_evens(
        &[70.0, 140.0],
        |price| price - 90.0,
        1e-8,
        40,
    )
    .expect("roots");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].direction, 1);
    assert!((roots[0].at - 90.0).abs() < 1e-6, "{}", roots[0].at);
}
