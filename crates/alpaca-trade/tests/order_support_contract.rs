#[path = "support/orders.rs"]
mod order_support;

use alpaca_trade::orders::OrderSide;
use order_support::{
    build_single_leg_context, find_call_broken_wing_butterfly, find_call_spread,
    find_distinct_call_spread_pair, find_iron_condor, find_put_spread, ObservedOptionContract,
    OptionContractType, QuotedOptionContract,
};
use rust_decimal::Decimal;

fn quoted(
    symbol: &str,
    expiration_date: &str,
    contract_type: OptionContractType,
    strike_price: i64,
    bid: i64,
    ask: i64,
) -> QuotedOptionContract {
    QuotedOptionContract {
        contract: ObservedOptionContract {
            symbol: symbol.to_owned(),
            expiration_date: expiration_date.to_owned(),
            contract_type,
            strike_price: Decimal::new(strike_price, 0),
        },
        bid: Decimal::new(bid, 1),
        ask: Decimal::new(ask, 1),
    }
}

#[test]
fn find_iron_condor_builds_a_debit_buy_strategy() {
    let spot = Decimal::new(102, 0);
    let puts = vec![
        quoted(
            "SPY250620P00095000",
            "2025-06-20",
            OptionContractType::Put,
            95,
            10,
            11,
        ),
        quoted(
            "SPY250620P00100000",
            "2025-06-20",
            OptionContractType::Put,
            100,
            20,
            21,
        ),
    ];
    let calls = vec![
        quoted(
            "SPY250620C00105000",
            "2025-06-20",
            OptionContractType::Call,
            105,
            20,
            21,
        ),
        quoted(
            "SPY250620C00110000",
            "2025-06-20",
            OptionContractType::Call,
            110,
            10,
            11,
        ),
    ];

    let context = find_iron_condor("SPY", spot, puts, calls)
        .expect("balanced quoted wings should produce a debit iron condor");

    assert_eq!(context.legs.len(), 4);
    assert_eq!(context.legs[0].side, Some(OrderSide::Sell));
    assert_eq!(context.legs[0].symbol, "SPY250620P00095000");
    assert_eq!(context.legs[1].side, Some(OrderSide::Buy));
    assert_eq!(context.legs[1].symbol, "SPY250620P00100000");
    assert_eq!(context.legs[2].side, Some(OrderSide::Buy));
    assert_eq!(context.legs[2].symbol, "SPY250620C00105000");
    assert_eq!(context.legs[3].side, Some(OrderSide::Sell));
    assert_eq!(context.legs[3].symbol, "SPY250620C00110000");
    assert!(context.marketable_limit_price > Decimal::ZERO);
}

#[test]
fn find_call_spread_returns_first_valid_orderable_pair() {
    let context = find_call_spread(
        "SPY",
        Decimal::new(94, 0),
        vec![
            quoted(
                "SPY250620C00090000",
                "2025-06-20",
                OptionContractType::Call,
                90,
                20,
                21,
            ),
            quoted(
                "SPY250620C00095000",
                "2025-06-20",
                OptionContractType::Call,
                95,
                10,
                11,
            ),
            quoted(
                "SPY250620C00100000",
                "2025-06-20",
                OptionContractType::Call,
                100,
                5,
                6,
            ),
        ],
    )
    .expect("first quoted debit call spread should be discoverable");

    assert_eq!(context.legs[0].symbol, "SPY250620C00090000");
    assert_eq!(context.legs[0].side, Some(OrderSide::Buy));
    assert_eq!(context.legs[1].symbol, "SPY250620C00095000");
    assert_eq!(context.legs[1].side, Some(OrderSide::Sell));
}

#[test]
fn find_put_spread_returns_first_valid_orderable_pair() {
    let context = find_put_spread(
        "SPY",
        Decimal::new(99, 0),
        vec![
            quoted(
                "SPY250620P00090000",
                "2025-06-20",
                OptionContractType::Put,
                90,
                5,
                6,
            ),
            quoted(
                "SPY250620P00095000",
                "2025-06-20",
                OptionContractType::Put,
                95,
                10,
                11,
            ),
            quoted(
                "SPY250620P00100000",
                "2025-06-20",
                OptionContractType::Put,
                100,
                20,
                21,
            ),
        ],
    )
    .expect("first quoted debit put spread should be discoverable");

    assert_eq!(context.legs[0].symbol, "SPY250620P00095000");
    assert_eq!(context.legs[0].side, Some(OrderSide::Buy));
    assert_eq!(context.legs[1].symbol, "SPY250620P00090000");
    assert_eq!(context.legs[1].side, Some(OrderSide::Sell));
}

#[test]
fn find_call_spread_skips_pairs_without_distinct_replace_price() {
    let context = find_call_spread(
        "SPY",
        Decimal::new(94, 0),
        vec![
            quoted(
                "SPY250620C00090000",
                "2025-06-20",
                OptionContractType::Call,
                90,
                1,
                2,
            ),
            quoted(
                "SPY250620C00095000",
                "2025-06-20",
                OptionContractType::Call,
                95,
                1,
                1,
            ),
            quoted(
                "SPY250620C00100000",
                "2025-06-20",
                OptionContractType::Call,
                100,
                0,
                0,
            ),
        ],
    )
    .expect("the helper should skip floor-colliding pairs and find a replaceable spread");

    assert_eq!(context.legs[0].symbol, "SPY250620C00095000");
    assert_eq!(context.legs[1].symbol, "SPY250620C00100000");
    assert!(context.more_conservative_limit_price < context.non_marketable_limit_price);
    assert!(context.deep_resting_limit_price < context.more_conservative_limit_price);
}

#[test]
fn find_call_spread_skips_pairs_without_strict_non_marketable_gap() {
    let context = find_call_spread(
        "SPY",
        Decimal::new(94, 0),
        vec![
            quoted(
                "SPY250620C00090000",
                "2025-06-20",
                OptionContractType::Call,
                90,
                10,
                11,
            ),
            quoted(
                "SPY250620C00095000",
                "2025-06-20",
                OptionContractType::Call,
                95,
                9,
                10,
            ),
            quoted(
                "SPY250620C00100000",
                "2025-06-20",
                OptionContractType::Call,
                100,
                5,
                6,
            ),
        ],
    )
    .expect("the helper should skip pairs whose mock midpoint collides with the floor");

    assert_eq!(context.legs[0].symbol, "SPY250620C00095000");
    assert_eq!(context.legs[1].symbol, "SPY250620C00100000");
}

#[test]
fn build_single_leg_context_rejects_indistinguishable_replace_price() {
    let error = build_single_leg_context(
        "SPY",
        quoted(
            "SPY250620C00095000",
            "2025-06-20",
            OptionContractType::Call,
            95,
            0,
            1,
        ),
    )
    .expect_err("floor-colliding single-leg quotes should be rejected");

    assert!(error.contains("distinct replace price"));
}

#[test]
fn find_iron_condor_ignores_spot_preference_when_a_valid_combo_exists() {
    let context = find_iron_condor(
        "SPY",
        Decimal::new(80, 0),
        vec![
            quoted(
                "SPY250620P00095000",
                "2025-06-20",
                OptionContractType::Put,
                95,
                10,
                11,
            ),
            quoted(
                "SPY250620P00100000",
                "2025-06-20",
                OptionContractType::Put,
                100,
                20,
                21,
            ),
        ],
        vec![
            quoted(
                "SPY250620C00105000",
                "2025-06-20",
                OptionContractType::Call,
                105,
                20,
                21,
            ),
            quoted(
                "SPY250620C00110000",
                "2025-06-20",
                OptionContractType::Call,
                110,
                10,
                11,
            ),
        ],
    )
    .expect("a valid quoted iron condor should not depend on proximity to spot");

    assert_eq!(context.legs.len(), 4);
    assert!(context.marketable_limit_price > Decimal::ZERO);
}

#[test]
fn find_call_broken_wing_butterfly_returns_121_structure() {
    let context = find_call_broken_wing_butterfly(
        "SPY",
        vec![
            quoted(
                "SPY250620C00095000",
                "2025-06-20",
                OptionContractType::Call,
                95,
                30,
                31,
            ),
            quoted(
                "SPY250620C00100000",
                "2025-06-20",
                OptionContractType::Call,
                100,
                20,
                21,
            ),
            quoted(
                "SPY250620C00110000",
                "2025-06-20",
                OptionContractType::Call,
                110,
                13,
                14,
            ),
        ],
    )
    .expect("quoted calls should produce a 1:2:1 broken wing butterfly");

    assert_eq!(context.legs.len(), 3);
    assert_eq!(context.legs[0].symbol, "SPY250620C00095000");
    assert_eq!(context.legs[0].ratio_qty, 1);
    assert_eq!(context.legs[0].side, Some(OrderSide::Buy));
    assert_eq!(context.legs[1].symbol, "SPY250620C00100000");
    assert_eq!(context.legs[1].ratio_qty, 2);
    assert_eq!(context.legs[1].side, Some(OrderSide::Sell));
    assert_eq!(context.legs[2].symbol, "SPY250620C00110000");
    assert_eq!(context.legs[2].ratio_qty, 1);
    assert_eq!(context.legs[2].side, Some(OrderSide::Buy));
}

#[test]
fn find_distinct_call_spread_pair_returns_two_disjoint_structures() {
    let (opened, replacement) = find_distinct_call_spread_pair(
        "SPY",
        vec![
            quoted(
                "SPY250620C00095000",
                "2025-06-20",
                OptionContractType::Call,
                95,
                30,
                31,
            ),
            quoted(
                "SPY250620C00100000",
                "2025-06-20",
                OptionContractType::Call,
                100,
                20,
                21,
            ),
            quoted(
                "SPY250620C00105000",
                "2025-06-20",
                OptionContractType::Call,
                105,
                12,
                13,
            ),
            quoted(
                "SPY250620C00110000",
                "2025-06-20",
                OptionContractType::Call,
                110,
                5,
                6,
            ),
        ],
    )
    .expect("quoted calls should produce two rollable call spreads");

    assert_eq!(opened.legs.len(), 2);
    assert_eq!(replacement.legs.len(), 2);
    assert_ne!(opened.legs[0].symbol, replacement.legs[0].symbol);
    assert_ne!(opened.legs[1].symbol, replacement.legs[1].symbol);
}
