use alpaca_option::payoff;
use alpaca_option::pricing;
use alpaca_option::{
    Greeks, OptionContract, OptionPosition, OptionQuote, OptionRight, OptionSnapshot,
    OptionStrategy, OptionStrategyInput, StrategyBreakEvenInput, StrategyPnlInput,
    StrategyValuationPosition,
};
use alpaca_time::expiration;
use rust_decimal::Decimal;

const TEST_RISK_FREE_RATE: f64 = 0.03;

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

fn strategy_position(
    expiration_date: &str,
    strike: f64,
    option_right: OptionRight,
    quantity: i32,
    avg_entry_price: f64,
    implied_volatility: f64,
) -> StrategyValuationPosition {
    StrategyValuationPosition {
        contract: contract(expiration_date, strike, option_right),
        quantity,
        avg_entry_price: Some(avg_entry_price),
        implied_volatility: Some(implied_volatility),
        mark_price: Some(avg_entry_price),
        reference_underlying_price: Some(100.0),
    }
}

fn option_position(
    expiration_date: &str,
    strike: f64,
    option_right: OptionRight,
    quantity: i32,
    greeks: Greeks,
) -> OptionPosition {
    let contract = contract(expiration_date, strike, option_right);
    OptionPosition {
        contract: contract.occ_symbol.clone(),
        snapshot: OptionSnapshot {
            as_of: "2025-03-20 10:30:00".to_string(),
            contract,
            quote: OptionQuote {
                bid: Some(1.0),
                ask: Some(1.2),
                mark: Some(1.1),
                last: Some(1.1),
            },
            greeks: Some(greeks),
            implied_volatility: Some(0.25),
            underlying_price: Some(100.0),
        },
        qty: quantity,
        avg_cost: Decimal::new(110, 2),
        leg_type: String::new(),
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
        rate: TEST_RISK_FREE_RATE,
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
        rate: TEST_RISK_FREE_RATE,
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
        rate: TEST_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.15,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let expected_short = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 95.0,
        years,
        rate: TEST_RISK_FREE_RATE,
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
        rate: TEST_RISK_FREE_RATE,
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
        rate: TEST_RISK_FREE_RATE,
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
        rate: TEST_RISK_FREE_RATE,
        dividend_yield: None,
        long_volatility_shift: None,
    })
    .unwrap_err();

    assert_eq!(error.code, "invalid_strategy_payoff_input");
}

#[test]
fn option_strategy_uses_earliest_expiration_close_by_default() {
    let positions = vec![
        strategy_position("2025-05-16", 100.0, OptionRight::Call, 1, 5.0, 0.24),
        strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 1.5, 0.31),
        strategy_position("2025-06-20", 90.0, OptionRight::Put, 1, 2.0, 0.28),
    ];

    assert_eq!(
        OptionStrategy::expiration_time(&positions).unwrap(),
        "2025-04-17 16:00:00"
    );

    let strategy = OptionStrategy::from_input(&OptionStrategyInput {
        positions,
        evaluation_time: None,
        entry_cost: None,
        rate: Some(TEST_RISK_FREE_RATE),
        dividend_yield: None,
        long_volatility_shift: None,
    })
    .unwrap();

    let direct = strategy.pnl_at(100.0).unwrap();
    let curve = strategy.sample_curve(90.0, 110.0, 10.0).unwrap();
    assert_eq!(curve.len(), 3);
    assert!((curve[1].pnl - direct).abs() < 1e-9);
}

#[test]
fn option_strategy_finds_break_even_between_bracketed_prices() {
    let strategy = OptionStrategy::prepare_with_rate(
        &[strategy_position(
            "2025-03-21",
            100.0,
            OptionRight::Call,
            1,
            5.0,
            0.30,
        )],
        "2025-03-21 16:00:00",
        None,
        TEST_RISK_FREE_RATE,
        None,
        None,
    )
    .unwrap();

    let root = strategy
        .break_even_between(100.0, 110.0, Some(1e-9), Some(100))
        .unwrap()
        .unwrap();
    assert!((root - 105.0).abs() < 1e-6, "root={root}");

    let no_root = strategy
        .break_even_between(90.0, 100.0, Some(1e-9), Some(100))
        .unwrap();
    assert_eq!(no_root, None);
}

#[test]
fn option_strategy_aggregates_snapshot_greeks_with_strategy_quantity() {
    let positions = vec![
        option_position(
            "2025-04-17",
            100.0,
            OptionRight::Call,
            1,
            Greeks {
                delta: 0.50,
                gamma: 0.01,
                vega: 0.08,
                theta: -0.03,
                rho: 0.02,
            },
        ),
        option_position(
            "2025-04-17",
            95.0,
            OptionRight::Put,
            -2,
            Greeks {
                delta: -0.25,
                gamma: 0.02,
                vega: 0.05,
                theta: -0.02,
                rho: -0.01,
            },
        ),
    ];

    let actual = OptionStrategy::aggregate_snapshot_greeks(&positions, 3.0).unwrap();

    assert!((actual.delta - 300.0).abs() < 1e-9);
    assert!((actual.gamma - -9.0).abs() < 1e-9);
    assert!((actual.vega - -6.0).abs() < 1e-9);
    assert!((actual.theta - 3.0).abs() < 1e-9);
    assert!((actual.rho - 12.0).abs() < 1e-9);
}

#[test]
fn option_strategy_aggregates_model_greeks_with_strategy_quantity() {
    let evaluation_time = "2025-03-20 11:30:04";
    let positions = vec![
        strategy_position("2025-04-17", 100.0, OptionRight::Call, 1, 4.0, 0.22),
        strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 1.5, 0.30),
    ];

    let actual = OptionStrategy::aggregate_model_greeks(
        &positions,
        102.0,
        evaluation_time,
        TEST_RISK_FREE_RATE,
        Some(0.0),
        None,
        2.0,
    )
    .unwrap();

    let years = expiration::years("2025-04-17", Some(evaluation_time), None);
    let long = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 100.0,
        years,
        rate: TEST_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.22,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let short = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 105.0,
        years,
        rate: TEST_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.30,
        option_right: OptionRight::Call,
    })
    .unwrap();

    assert!((actual.delta - (long.delta - short.delta) * 200.0).abs() < 1e-9);
    assert!((actual.gamma - (long.gamma - short.gamma) * 200.0).abs() < 1e-9);
    assert!((actual.vega - (long.vega - short.vega) * 200.0).abs() < 1e-9);
    assert!((actual.theta - (long.theta - short.theta) * 200.0).abs() < 1e-9);
    assert!((actual.rho - (long.rho - short.rho) * 200.0).abs() < 1e-9);
}

#[test]
fn option_strategy_values_common_multi_leg_shapes() {
    let cases: [(&str, Vec<StrategyValuationPosition>, f64); 6] = [
        (
            "pmcc",
            vec![
                strategy_position("2025-06-20", 95.0, OptionRight::Call, 1, 12.0, 0.24),
                strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 2.0, 0.30),
            ],
            105.0,
        ),
        (
            "double_diagonal",
            vec![
                strategy_position("2025-05-16", 90.0, OptionRight::Put, 1, 2.2, 0.27),
                strategy_position("2025-04-17", 95.0, OptionRight::Put, -1, 1.4, 0.32),
                strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 1.5, 0.31),
                strategy_position("2025-05-16", 110.0, OptionRight::Call, 1, 2.4, 0.26),
            ],
            100.0,
        ),
        (
            "broken_wing_plus_diagonal",
            vec![
                strategy_position("2025-04-17", 100.0, OptionRight::Call, 1, 4.0, 0.25),
                strategy_position("2025-04-17", 105.0, OptionRight::Call, -2, 2.0, 0.27),
                strategy_position("2025-04-17", 115.0, OptionRight::Call, 1, 0.6, 0.30),
                strategy_position("2025-04-17", 95.0, OptionRight::Put, -1, 1.1, 0.30),
                strategy_position("2025-05-16", 90.0, OptionRight::Put, 1, 1.4, 0.25),
            ],
            105.0,
        ),
        (
            "iron_condor",
            vec![
                strategy_position("2025-04-17", 90.0, OptionRight::Put, 1, 0.6, 0.30),
                strategy_position("2025-04-17", 95.0, OptionRight::Put, -1, 1.3, 0.30),
                strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 1.2, 0.30),
                strategy_position("2025-04-17", 110.0, OptionRight::Call, 1, 0.5, 0.30),
            ],
            100.0,
        ),
        (
            "short_straddle",
            vec![
                strategy_position("2025-04-17", 100.0, OptionRight::Call, -1, 3.0, 0.30),
                strategy_position("2025-04-17", 100.0, OptionRight::Put, -1, 2.8, 0.30),
            ],
            100.0,
        ),
        (
            "call_butterfly",
            vec![
                strategy_position("2025-04-17", 95.0, OptionRight::Call, 1, 6.0, 0.30),
                strategy_position("2025-04-17", 100.0, OptionRight::Call, -2, 3.0, 0.30),
                strategy_position("2025-04-17", 105.0, OptionRight::Call, 1, 1.0, 0.30),
            ],
            100.0,
        ),
    ];

    for (name, positions, pivot) in cases {
        let strategy = OptionStrategy::from_input(&OptionStrategyInput {
            positions,
            evaluation_time: None,
            entry_cost: None,
            rate: Some(TEST_RISK_FREE_RATE),
            dividend_yield: None,
            long_volatility_shift: None,
        })
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        let pivot_pnl = strategy
            .pnl_at(pivot)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(pivot_pnl.is_finite(), "{name}: non-finite pivot pnl");

        let curve = strategy
            .sample_curve(f64::max(pivot * 0.8, 1.0), pivot * 1.2, pivot * 0.1)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(curve.len() >= 5, "{name}: curve too small");
        assert!(
            curve
                .iter()
                .all(|point| point.underlying_price.is_finite()
                    && point.mark_value.is_finite()
                    && point.pnl.is_finite()),
            "{name}: curve contains non-finite point"
        );
    }
}
