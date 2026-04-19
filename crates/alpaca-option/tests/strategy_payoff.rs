use alpaca_option::payoff;
use alpaca_option::pricing;
use alpaca_option::{
    OptionContract, OptionRight, StrategyBreakEvenInput, StrategyPnlInput,
    StrategyValuationPosition,
};
use alpaca_time::expiration;

fn contract(expiration_date: &str, strike: f64, option_right: OptionRight) -> OptionContract {
    let right_code = match option_right {
        OptionRight::Call => "C",
        OptionRight::Put => "P",
    };
    let occ_strike = format!("{:08}", (strike * 1000.0).round() as i64);
    let compact_expiration = expiration_date.replace('-', "");
    let compact_expiration = &compact_expiration[2..];

    OptionContract {
        underlying_symbol: "SPY".to_string(),
        expiration_date: expiration_date.to_string(),
        strike,
        option_right,
        occ_symbol: format!("SPY{compact_expiration}{right_code}{occ_strike}"),
    }
}

#[test]
fn strategy_pnl_mixes_expired_and_unexpired_positions() {
    let evaluation_time = "2025-03-21 16:00:00";
    let positions = vec![
        StrategyValuationPosition {
            contract: contract("2025-03-21", 100.0, OptionRight::Put),
            quantity: -1,
            avg_entry_price: Some(2.0),
            implied_volatility: Some(0.30),
            mark_price: None,
            reference_underlying_price: None,
        },
        StrategyValuationPosition {
            contract: contract("2025-04-24", 95.0, OptionRight::Put),
            quantity: 1,
            avg_entry_price: Some(1.0),
            implied_volatility: Some(0.25),
            mark_price: None,
            reference_underlying_price: None,
        },
    ];

    let expected_long_value = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 97.0,
        strike: 95.0,
        years: expiration::years("2025-04-24", Some(evaluation_time), None),
        rate: 0.03,
        dividend_yield: 0.0,
        volatility: 0.25,
        option_right: OptionRight::Put,
    })
    .unwrap();
    let expected = (expected_long_value - 3.0) * 100.0 + 100.0;

    let actual = payoff::strategy_pnl(&StrategyPnlInput {
        positions,
        underlying_price: 97.0,
        evaluation_time: evaluation_time.to_string(),
        entry_cost: None,
        rate: 0.03,
        dividend_yield: None,
        long_volatility_shift: None,
    })
    .unwrap();

    assert!(
        (actual - expected).abs() < 1e-9,
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn strategy_pnl_applies_volatility_shift_only_to_long_positions() {
    let evaluation_time = "2025-03-20 11:30:04";
    let positions = vec![
        StrategyValuationPosition {
            contract: contract("2025-04-24", 100.0, OptionRight::Call),
            quantity: 1,
            avg_entry_price: Some(2.5),
            implied_volatility: Some(0.20),
            mark_price: None,
            reference_underlying_price: None,
        },
        StrategyValuationPosition {
            contract: contract("2025-04-24", 95.0, OptionRight::Put),
            quantity: -1,
            avg_entry_price: Some(1.0),
            implied_volatility: Some(0.30),
            mark_price: None,
            reference_underlying_price: None,
        },
    ];

    let years = expiration::years("2025-04-24", Some(evaluation_time), None);
    let expected_long = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 100.0,
        years,
        rate: 0.03,
        dividend_yield: 0.0,
        volatility: 0.15,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let expected_short = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 95.0,
        years,
        rate: 0.03,
        dividend_yield: 0.0,
        volatility: 0.30,
        option_right: OptionRight::Put,
    })
    .unwrap();
    let expected = (expected_long - expected_short) * 100.0 - 150.0;

    let actual = payoff::strategy_pnl(&StrategyPnlInput {
        positions,
        underlying_price: 102.0,
        evaluation_time: evaluation_time.to_string(),
        entry_cost: Some(150.0),
        rate: 0.03,
        dividend_yield: None,
        long_volatility_shift: Some(-0.05),
    })
    .unwrap();

    assert!(
        (actual - expected).abs() < 1e-9,
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn strategy_break_even_points_finds_credit_strangle_roots() {
    let actual = payoff::strategy_break_even_points(&StrategyBreakEvenInput {
        positions: vec![
            StrategyValuationPosition {
                contract: contract("2025-03-21", 90.0, OptionRight::Put),
                quantity: -1,
                avg_entry_price: Some(1.5),
                implied_volatility: Some(0.25),
                mark_price: None,
                reference_underlying_price: None,
            },
            StrategyValuationPosition {
                contract: contract("2025-03-21", 110.0, OptionRight::Call),
                quantity: -1,
                avg_entry_price: Some(1.5),
                implied_volatility: Some(0.25),
                mark_price: None,
                reference_underlying_price: None,
            },
        ],
        evaluation_time: "2025-03-21 16:00:00".to_string(),
        entry_cost: None,
        rate: 0.03,
        dividend_yield: None,
        long_volatility_shift: None,
        lower_bound: 50.0,
        upper_bound: 150.0,
        scan_step: Some(1.0),
        tolerance: Some(1e-9),
        max_iterations: Some(100),
    })
    .unwrap();

    assert_eq!(actual.len(), 2);
    assert!((actual[0] - 87.0).abs() < 1e-6, "actual={actual:?}");
    assert!((actual[1] - 113.0).abs() < 1e-6, "actual={actual:?}");
}

#[test]
fn strategy_pnl_requires_entry_cost_or_leg_costs() {
    let error = payoff::strategy_pnl(&StrategyPnlInput {
        positions: vec![StrategyValuationPosition {
            contract: contract("2025-04-24", 100.0, OptionRight::Call),
            quantity: 1,
            avg_entry_price: None,
            implied_volatility: Some(0.20),
            mark_price: None,
            reference_underlying_price: None,
        }],
        underlying_price: 102.0,
        evaluation_time: "2025-03-20 11:30:04".to_string(),
        entry_cost: None,
        rate: 0.03,
        dividend_yield: None,
        long_volatility_shift: None,
    })
    .unwrap_err();

    assert_eq!(error.code, "invalid_strategy_payoff_input");
}
