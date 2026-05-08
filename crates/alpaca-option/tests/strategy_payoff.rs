use alpaca_option::option_strategy;
use alpaca_option::pricing;
use alpaca_option::{
    Greeks, OptionContract, OptionPosition, OptionQuote, OptionRight, OptionSnapshot,
    OptionStrategy, OptionStrategyInput, StrategyBreakEvenInput, StrategyBreakEvenSideInput,
    StrategyPnlInput, StrategyPnlPeakSearchInput, DEFAULT_RISK_FREE_RATE,
};
use alpaca_time::expiration;
use rust_decimal::Decimal;

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
) -> OptionPosition {
    let contract = contract(expiration_date, strike, option_right);
    OptionPosition {
        contract: contract.occ_symbol.clone(),
        snapshot: OptionSnapshot {
            as_of: "2025-03-20 10:30:00".to_string(),
            contract,
            quote: OptionQuote {
                bid: Some(avg_entry_price),
                ask: Some(avg_entry_price),
                mark: Some(avg_entry_price),
                last: Some(avg_entry_price),
            },
            greeks: None,
            implied_volatility: Some(implied_volatility),
            underlying_price: Some(100.0),
        },
        qty: quantity,
        avg_cost: alpaca_core::decimal::from_f64(avg_entry_price, 4),
        leg_type: String::new(),
        option_right: None,
        strike: None,
        valuation_years: None,
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
        option_right: None,
        strike: None,
        valuation_years: None,
    }
}

fn priced_position(
    expiration_date: &str,
    strike: f64,
    option_right: OptionRight,
    quantity: i32,
    avg_entry_price: f64,
    bid: f64,
    ask: f64,
    mark: f64,
) -> OptionPosition {
    let contract = contract(expiration_date, strike, option_right);
    OptionPosition {
        contract: contract.occ_symbol.clone(),
        snapshot: OptionSnapshot {
            as_of: "2025-03-20 10:30:00".to_string(),
            contract,
            quote: OptionQuote {
                bid: Some(bid),
                ask: Some(ask),
                mark: Some(mark),
                last: Some(mark),
            },
            greeks: None,
            implied_volatility: Some(0.25),
            underlying_price: Some(100.0),
        },
        qty: quantity,
        avg_cost: alpaca_core::decimal::from_f64(avg_entry_price, 4),
        leg_type: String::new(),
        option_right: None,
        strike: None,
        valuation_years: None,
    }
}

#[test]
fn option_strategy_position_totals_use_instance_qty_and_enrich_positions() {
    let long = priced_position(
        "2026-05-15",
        450.0,
        OptionRight::Call,
        2,
        1.00,
        1.10,
        1.30,
        1.20,
    );
    let short = priced_position(
        "2026-05-15",
        455.0,
        OptionRight::Call,
        -1,
        0.55,
        0.40,
        0.60,
        0.50,
    );

    let strategy = OptionStrategy::prepare(
        &[long, short],
        3,
        "2026-05-15 16:00:00",
        None,
        Some(0.0),
    )
    .unwrap();
    let totals = strategy.position_totals();

    assert_eq!(totals.value, Decimal::new(57000, 2));
    assert_eq!(totals.cost, Decimal::new(43500, 2));
    assert_eq!(totals.spread, Decimal::new(18000, 2));
    assert!((totals.spread_rate.unwrap() - (180.0 / 435.0)).abs() < 1e-12);

    let positions = strategy.positions();
    assert_eq!(positions[0].option_right, Some(OptionRight::Call));
    assert_eq!(positions[0].strike, Some(450.0));
    assert_eq!(positions[0].valuation_years, Some(0.0));
    assert_eq!(positions[0].snapshot.implied_volatility, Some(0.25));
}

#[test]
fn option_strategy_exposes_serializable_state_fields() {
    let long = priced_position(
        "2026-05-15",
        450.0,
        OptionRight::Call,
        2,
        1.00,
        1.10,
        1.30,
        1.20,
    );
    let short = priced_position(
        "2026-05-15",
        455.0,
        OptionRight::Call,
        -1,
        0.55,
        0.40,
        0.60,
        0.50,
    );

    let mut strategy = OptionStrategy::prepare(
        &[long, short],
        3,
        "2026-05-15 16:00:00",
        None,
        Some(0.0),
    )
    .unwrap();
    strategy.underlying_price = 452.0;
    strategy.calculate_position_totals();

    assert_eq!(strategy.qty, 3);
    assert_eq!(strategy.positions.len(), 2);
    assert_eq!(strategy.value, Decimal::new(57000, 2));
    assert_eq!(strategy.cost, Decimal::new(43500, 2));
    assert_eq!(strategy.spread, Some(Decimal::new(18000, 2)));
    assert!((strategy.spread_rate.unwrap() - (180.0 / 435.0)).abs() < 1e-12);

    let json = serde_json::to_value(&strategy).unwrap();
    assert!(json.get("positions").is_some());
    assert!(json.get("qty").is_some());
    assert!(json.get("underlying_price").is_some());
    assert!(json.get("current_underlying_price").is_none());
}

#[test]
fn option_position_helpers_prepare_runtime_model_inputs() {
    let resolved_contract = contract("2026-05-15", 450.0, OptionRight::Call);
    let mut snapshot = OptionSnapshot::default();
    snapshot.contract = resolved_contract;
    snapshot.quote.mark = Some(2.34);

    let position = OptionPosition::from_snapshot(&snapshot, 1, Decimal::new(234, 2), "longcall");

    assert_eq!(position.contract, "SPY260515C00450000");
    assert_eq!(position.qty, 1);
    assert_eq!(position.leg_type, "longcall");
    assert_eq!(position.avg_cost, Decimal::new(234, 2));

    let modeled = position
        .with_qty_multiplier(3)
        .with_model_inputs(0.31, Some(451.25));

    assert_eq!(modeled.qty, 3);
    assert_eq!(modeled.snapshot.implied_volatility, Some(0.31));
    assert_eq!(modeled.snapshot.underlying_price, Some(451.25));
    assert_eq!(modeled.effective_iv_or(Some(0.25), 0.30), 0.31);
}

#[test]
fn option_position_effective_iv_uses_fallback_then_default() {
    let mut position =
        option_position("2026-05-15", 450.0, OptionRight::Call, 1, Greeks::default());
    position.snapshot.implied_volatility = None;

    assert_eq!(position.effective_iv_or(Some(0.22), 0.30), 0.22);
    assert_eq!(position.effective_iv_or(None, 0.30), 0.30);
}

#[test]
fn strategy_pnl_mixes_expired_and_unexpired_positions() {
    let evaluation_time = "2025-03-21 16:00:00";
    let positions = vec![
        strategy_position("2025-03-21", 100.0, OptionRight::Put, -1, 2.0, 0.30),
        strategy_position("2025-04-24", 95.0, OptionRight::Put, 1, 1.0, 0.25),
    ];

    let expected_long_value = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 97.0,
        strike: 95.0,
        years: expiration::years("2025-04-24", Some(evaluation_time), None),
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.25,
        option_right: OptionRight::Put,
    })
    .unwrap();
    let expected = (expected_long_value - 3.0) * 100.0 + 100.0;

    let actual = option_strategy::strategy_pnl(&StrategyPnlInput {
        positions,
        qty: 1,
        underlying_price: 97.0,
        evaluation_time: evaluation_time.to_string(),
        entry_cost: None,
        dividend_yield: None,
    })
    .unwrap();

    assert!(
        (actual - expected).abs() < 1e-9,
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn strategy_pnl_uses_snapshot_implied_volatility_directly() {
    let evaluation_time = "2025-03-20 11:30:04";
    let positions = vec![
        strategy_position("2025-04-24", 100.0, OptionRight::Call, 1, 2.5, 0.20),
        strategy_position("2025-04-24", 95.0, OptionRight::Put, -1, 1.0, 0.30),
    ];

    let years = expiration::years("2025-04-24", Some(evaluation_time), None);
    let expected_long = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 100.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.20,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let expected_short = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 95.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.30,
        option_right: OptionRight::Put,
    })
    .unwrap();
    let expected = (expected_long - expected_short) * 100.0 - 150.0;

    let actual = option_strategy::strategy_pnl(&StrategyPnlInput {
        positions,
        qty: 1,
        underlying_price: 102.0,
        evaluation_time: evaluation_time.to_string(),
        entry_cost: Some(150.0),
        dividend_yield: None,
    })
    .unwrap();

    assert!(
        (actual - expected).abs() < 1e-9,
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn strategy_break_even_points_finds_credit_strangle_roots() {
    let actual = option_strategy::strategy_break_even_points(&StrategyBreakEvenInput {
        positions: vec![
            strategy_position("2025-03-21", 90.0, OptionRight::Put, -1, 1.5, 0.25),
            strategy_position("2025-03-21", 110.0, OptionRight::Call, -1, 1.5, 0.25),
        ],
        qty: 1,
        evaluation_time: "2025-03-21 16:00:00".to_string(),
        entry_cost: None,
        dividend_yield: None,
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
fn strategy_pnl_requires_snapshot_iv_before_expiration() {
    let mut position = strategy_position("2025-04-24", 100.0, OptionRight::Call, 1, 0.0, 0.20);
    position.snapshot = OptionSnapshot::default();

    let error = option_strategy::strategy_pnl(&StrategyPnlInput {
        positions: vec![position],
        qty: 1,
        underlying_price: 102.0,
        evaluation_time: "2025-03-20 11:30:04".to_string(),
        entry_cost: None,
        dividend_yield: None,
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
        qty: 1,
        evaluation_time: None,
        entry_cost: None,
        dividend_yield: None,
    })
    .unwrap();

    let direct = strategy.pnl_at(100.0).unwrap();
    let curve = strategy.sample_curve(90.0, 110.0, 10.0).unwrap();
    assert_eq!(curve.len(), 3);
    assert!((curve[1].pnl - direct).abs() < 1e-9);
}

#[test]
fn option_strategy_finds_break_even_between_bracketed_prices() {
    let strategy = OptionStrategy::prepare(
        &[strategy_position(
            "2025-03-21",
            100.0,
            OptionRight::Call,
            1,
            5.0,
            0.30,
        )],
        1,
        "2025-03-21 16:00:00",
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
fn option_strategy_single_side_break_even_helpers_find_roots() {
    let positions = vec![
        strategy_position("2025-03-21", 90.0, OptionRight::Put, -1, 1.5, 0.25),
        strategy_position("2025-03-21", 110.0, OptionRight::Call, -1, 1.5, 0.25),
    ];
    let strategy =
        OptionStrategy::prepare(&positions, 1, "2025-03-21 16:00:00", None, Some(0.0)).unwrap();

    let left = strategy
        .find_break_even_left(&StrategyBreakEvenSideInput {
            pivot: 90.0,
            boundary: 50.0,
            scan_step: 1.0,
            tolerance: Some(1e-9),
            max_iterations: Some(100),
        })
        .unwrap();
    let right = strategy
        .find_break_even_right(&StrategyBreakEvenSideInput {
            pivot: 110.0,
            boundary: 150.0,
            scan_step: 1.0,
            tolerance: Some(1e-9),
            max_iterations: Some(100),
        })
        .unwrap();

    assert!((left.unwrap() - 87.0).abs() < 1e-6);
    assert!((right.unwrap() - 113.0).abs() < 1e-6);
}

#[test]
fn unique_break_even_points_sorts_and_dedupes() {
    let points =
        option_strategy::unique_break_even_points([402.0, 401.9999999, 398.0, f64::NAN], 1e-6);
    assert_eq!(points, vec![398.0, 402.0]);
}

#[test]
fn option_strategy_pnl_peak_from_current_finds_positive_peak() {
    let positions = vec![
        strategy_position("2025-03-21", 90.0, OptionRight::Put, -1, 4.0, 0.25),
        strategy_position("2025-03-21", 110.0, OptionRight::Call, -1, 4.0, 0.25),
    ];
    let strategy =
        OptionStrategy::prepare(&positions, 1, "2025-03-21 16:00:00", None, Some(0.0)).unwrap();

    let peak = strategy
        .pnl_peak_from_current(&StrategyPnlPeakSearchInput {
            current_price: 100.0,
            step_hint: Some(1.0),
            left_boundary: 1.0,
            right_boundary: 300.0,
            tolerance: Some(1e-9),
            max_search_steps: Some(512),
        })
        .unwrap()
        .unwrap();

    assert!(peak.spot.is_finite());
    assert!(peak.pnl > 0.0);
}

#[test]
fn option_strategy_prepare_preserves_snapshot_implied_volatility() {
    let mut position = strategy_position("2026-05-15", 450.0, OptionRight::Call, 1, 2.50, 0.10);
    position.snapshot.quote.mark = Some(8.00);
    position.snapshot.underlying_price = Some(452.0);

    let strategy =
        OptionStrategy::prepare(&[position], 1, "2026-05-01 10:00:00", Some(250.0), Some(0.0))
            .unwrap();

    let modeled = strategy.positions();
    assert_eq!(modeled.len(), 1);
    assert_eq!(modeled[0].snapshot.implied_volatility, Some(0.10));
}

#[test]
fn option_strategy_aggregates_snapshot_greeks_with_qty() {
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

    let actual = OptionStrategy::aggregate_snapshot_greeks(&positions, 3).unwrap();

    assert!((actual.delta - 300.0).abs() < 1e-9);
    assert!((actual.gamma - -9.0).abs() < 1e-9);
    assert!((actual.vega - -6.0).abs() < 1e-9);
    assert!((actual.theta - 3.0).abs() < 1e-9);
    assert!((actual.rho - 12.0).abs() < 1e-9);
}

#[test]
fn option_strategy_aggregates_model_greeks_with_qty() {
    let evaluation_time = "2025-03-20 11:30:04";
    let positions = vec![
        strategy_position("2025-04-17", 100.0, OptionRight::Call, 1, 4.0, 0.22),
        strategy_position("2025-04-17", 105.0, OptionRight::Call, -1, 1.5, 0.30),
    ];

    let actual = OptionStrategy::aggregate_model_greeks(
        &positions,
        102.0,
        evaluation_time,
        Some(0.0),
        2,
    )
    .unwrap();
    let direct = OptionStrategy::prepare(&positions, 2, evaluation_time, Some(0.0), Some(0.0))
        .unwrap()
        .greeks_at(102.0)
        .unwrap();
    assert_eq!(actual, direct);

    let years = expiration::years("2025-04-17", Some(evaluation_time), None);
    let long = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 100.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.22,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let short = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 105.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
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
fn option_strategy_prepares_from_option_positions_and_uses_instance_greeks() {
    let evaluation_time = "2025-03-20 11:30:04";
    let positions = vec![
        option_position("2025-04-17", 100.0, OptionRight::Call, 1, Greeks::default()),
        option_position(
            "2025-04-17",
            105.0,
            OptionRight::Call,
            -1,
            Greeks::default(),
        ),
    ];

    let strategy =
        OptionStrategy::prepare(&positions, 2, evaluation_time, None, Some(0.0)).unwrap();
    let actual = strategy.greeks_at(102.0).unwrap();

    let years = expiration::years("2025-04-17", Some(evaluation_time), None);
    let long = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 100.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.25,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let short = pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
        spot: 102.0,
        strike: 105.0,
        years,
        rate: DEFAULT_RISK_FREE_RATE,
        dividend_yield: 0.0,
        volatility: 0.25,
        option_right: OptionRight::Call,
    })
    .unwrap();

    assert!((actual.delta - (long.delta - short.delta) * 200.0).abs() < 1e-9);
}

#[test]
fn option_strategy_values_common_multi_leg_shapes() {
    let cases: [(&str, Vec<OptionPosition>, f64); 6] = [
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
            qty: 1,
            evaluation_time: None,
            entry_cost: None,
            dividend_yield: None,
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
            curve.iter().all(|point| point.underlying_price.is_finite()
                && point.mark_value.is_finite()
                && point.pnl.is_finite()),
            "{name}: curve contains non-finite point"
        );
    }
}
