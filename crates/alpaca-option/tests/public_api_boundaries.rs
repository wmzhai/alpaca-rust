use alpaca_core::decimal;
use alpaca_option::analysis;
use alpaca_option::chain;
use alpaca_option::contract;
use alpaca_option::display;
use alpaca_option::execution_quote;
use alpaca_option::math;
use alpaca_option::numeric;
use alpaca_option::payoff;
use alpaca_option::pricing;
use alpaca_option::probability;
use alpaca_option::snapshot;
use alpaca_option::types::{
    ContractDisplay, ExecutionSnapshot, Greeks, OptionChainRecord, OptionContract, OptionPosition,
    OptionQuote, OptionRight, OptionRightCode, OptionSnapshot,
};
use alpaca_option::url;
use alpaca_option::{
    LiquidityData, LiquidityOptionData, LiquidityStats, OptionError, PayoffLegInput,
};
use serde_json::json;
use ts_rs::TS;

fn assert_error_code(error: OptionError, expected: &str) {
    assert_eq!(error.code, expected, "unexpected error: {error}");
}

fn sample_contract() -> OptionContract {
    OptionContract {
        underlying_symbol: "SPY".to_string(),
        expiration_date: "2025-03-21".to_string(),
        strike: 600.0,
        option_right: OptionRight::Call,
        occ_symbol: "SPY250321C00600000".to_string(),
    }
}

fn contract_at(strike: f64, option_right: OptionRight) -> OptionContract {
    let occ_symbol = match option_right {
        OptionRight::Call => match strike {
            100.0 => "SPY250321C00100000",
            110.0 => "SPY250321C00110000",
            _ => "SPY250321C00600000",
        },
        OptionRight::Put => match strike {
            95.0 => "SPY250321P00095000",
            110.0 => "SPY250321P00110000",
            _ => "SPY250321P00580000",
        },
    };

    OptionContract {
        underlying_symbol: "SPY".to_string(),
        expiration_date: "2025-03-21".to_string(),
        strike,
        option_right,
        occ_symbol: occ_symbol.to_string(),
    }
}

fn sample_snapshot(bid: Option<f64>, ask: Option<f64>) -> OptionSnapshot {
    OptionSnapshot {
        as_of: "2025-02-06 11:30:04".to_string(),
        contract: sample_contract(),
        quote: OptionQuote {
            bid,
            ask,
            mark: match (bid, ask) {
                (Some(left), Some(right)) => Some((left + right) / 2.0),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            },
            last: None,
        },
        greeks: None,
        implied_volatility: None,
        underlying_price: None,
    }
}

fn sample_position(
    contract: OptionContract,
    qty: i32,
    avg_cost: Option<f64>,
    leg_type: &str,
    snapshot: Option<OptionSnapshot>,
) -> OptionPosition {
    OptionPosition {
        contract: contract.occ_symbol.clone(),
        snapshot: snapshot.unwrap_or_default(),
        qty,
        avg_cost: decimal::from_f64(avg_cost.unwrap_or(0.0), 2),
        leg_type: leg_type.to_string(),
    }
}

#[test]
fn snapshot_core_accessors_and_execution_snapshot_bridge_stay_in_bottom_library() {
    let empty = OptionSnapshot::default();
    assert!(empty.is_empty());
    assert_eq!(empty.bid(), 0.0);
    assert_eq!(empty.ask(), 0.0);
    assert_eq!(empty.price(), 0.0);
    assert_eq!(empty.iv(), 0.0);

    let execution_snapshot = ExecutionSnapshot {
        contract: "SPY250321C00600000".to_string(),
        timestamp: "2025-02-06 11:30:04".to_string(),
        bid: "1.1".to_string(),
        ask: "1.3".to_string(),
        price: "1.2".to_string(),
        greeks: Greeks {
            delta: 0.25,
            gamma: 0.03,
            vega: 0.11,
            theta: -0.05,
            rho: 0.01,
        },
        iv: 0.22,
    };

    let canonical = OptionSnapshot::from(execution_snapshot.clone());
    assert!(!canonical.is_empty());
    assert_eq!(canonical.occ_symbol(), "SPY250321C00600000");
    assert_eq!(canonical.timestamp(), "2025-02-06 11:30:04");
    assert_eq!(canonical.bid(), 1.1);
    assert_eq!(canonical.ask(), 1.3);
    assert_eq!(canonical.price(), 1.2);
    assert_eq!(canonical.iv(), 0.22);
    assert_eq!(canonical.greeks_or_default().delta, 0.25);

    let bridged = ExecutionSnapshot::from(&canonical);
    assert_eq!(bridged, execution_snapshot);
}

#[test]
fn option_chain_record_roundtrip_stays_in_bottom_library() {
    let snapshot = OptionSnapshot {
        as_of: "2025-02-06 11:30:04".to_string(),
        contract: sample_contract(),
        quote: OptionQuote {
            bid: Some(1.1),
            ask: Some(1.3),
            mark: Some(1.2),
            last: Some(1.2),
        },
        greeks: Some(Greeks {
            delta: 0.25,
            gamma: 0.03,
            vega: 0.11,
            theta: -0.05,
            rho: 0.01,
        }),
        implied_volatility: Some(0.22),
        underlying_price: Some(598.75),
    };

    let record = OptionChainRecord::from(&snapshot);
    assert_eq!(record.as_of, "2025-02-06 11:30:04");
    assert_eq!(record.underlying_symbol, "SPY");
    assert_eq!(record.occ_symbol, "SPY250321C00600000");
    assert_eq!(record.strike, 600.0);
    assert_eq!(record.underlying_price, Some(598.75));
    assert_eq!(record.bid, Some(1.1));
    assert_eq!(record.ask, Some(1.3));
    assert_eq!(record.mark, Some(1.2));
    assert_eq!(record.implied_volatility, Some(0.22));
    assert_eq!(record.delta, Some(0.25));

    let bridged = OptionSnapshot::from(&record);
    assert_eq!(bridged.occ_symbol(), "SPY250321C00600000");
    assert_eq!(bridged.timestamp(), "2025-02-06 11:30:04");
    assert_eq!(bridged.bid(), 1.1);
    assert_eq!(bridged.ask(), 1.3);
    assert_eq!(bridged.price(), 1.2);
    assert_eq!(bridged.iv(), 0.22);
    assert_eq!(bridged.greeks_or_default().delta, 0.25);
    assert_eq!(bridged.underlying_price, Some(598.75));
}

#[test]
fn canonical_option_chain_queries_stay_in_bottom_library() {
    let chain = alpaca_option::OptionChain {
        underlying_symbol: "SPY".to_string(),
        as_of: "2026-04-17 10:00:00".to_string(),
        snapshots: vec![
            OptionSnapshot::from(ExecutionSnapshot {
                contract: "SPY260515C00600000".to_string(),
                timestamp: "2026-04-17 10:00:00".to_string(),
                bid: "2.00".to_string(),
                ask: "2.20".to_string(),
                price: "2.10".to_string(),
                greeks: Greeks::default(),
                iv: 0.25,
            }),
            OptionSnapshot::from(ExecutionSnapshot {
                contract: "SPY260515P00580000".to_string(),
                timestamp: "2026-04-17 10:00:00".to_string(),
                bid: "1.10".to_string(),
                ask: "1.30".to_string(),
                price: "1.20".to_string(),
                greeks: Greeks::default(),
                iv: 0.21,
            }),
            OptionSnapshot::from(ExecutionSnapshot {
                contract: "SPY260619C00610000".to_string(),
                timestamp: "2026-04-17 10:00:00".to_string(),
                bid: "1.40".to_string(),
                ask: "1.60".to_string(),
                price: "1.50".to_string(),
                greeks: Greeks::default(),
                iv: 0.24,
            }),
        ],
    };

    let expirations = chain::expiration_dates(
        &chain,
        None,
        Some(20),
        Some(80),
        Some("2026-04-17 10:00:00"),
    )
    .expect("expiration dates should resolve");
    assert_eq!(
        expirations
            .into_iter()
            .map(|item| item.expiration_date)
            .collect::<Vec<_>>(),
        vec!["2026-05-15".to_string(), "2026-06-19".to_string()]
    );

    let may_calls = chain::list_snapshots(
        &chain,
        chain::SnapshotFilter {
            expiration_date: Some("2026-05-15"),
            option_right: Some("call"),
            ..Default::default()
        },
    );
    assert_eq!(may_calls.len(), 1);
    assert_eq!(may_calls[0].occ_symbol(), "SPY260515C00600000");

    let exact = chain::find_snapshot(
        &chain,
        chain::SnapshotFilter {
            expiration_date: Some("2026-05-15"),
            option_right: Some("put"),
            strike: Some(580.0),
            ..Default::default()
        },
    )
    .expect("should find exact snapshot");
    assert_eq!(exact.occ_symbol(), "SPY260515P00580000");
}

#[test]
fn canonical_snapshot_serialization_keeps_single_canonical_shape_in_bottom_library() {
    let snapshot = OptionSnapshot {
        as_of: "2025-02-06 11:30:04".to_string(),
        contract: sample_contract(),
        quote: OptionQuote {
            bid: Some(1.1),
            ask: Some(1.3),
            mark: Some(1.2),
            last: Some(1.2),
        },
        greeks: Some(Greeks {
            delta: 0.25,
            gamma: 0.03,
            vega: 0.11,
            theta: -0.05,
            rho: 0.01,
        }),
        implied_volatility: Some(0.22),
        underlying_price: Some(598.75),
    };

    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(
        encoded,
        json!({
            "as_of": "2025-02-06 11:30:04",
            "contract": {
                "underlying_symbol": "SPY",
                "expiration_date": "2025-03-21",
                "strike": 600.0,
                "option_right": "call",
                "occ_symbol": "SPY250321C00600000"
            },
            "quote": {
                "bid": 1.1,
                "ask": 1.3,
                "mark": 1.2,
                "last": 1.2
            },
            "greeks": {
                "delta": 0.25,
                "gamma": 0.03,
                "vega": 0.11,
                "theta": -0.05,
                "rho": 0.01
            },
            "implied_volatility": 0.22,
            "underlying_price": 598.75
        })
    );

    let decoded: OptionSnapshot = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.occ_symbol(), "SPY250321C00600000");
    assert_eq!(decoded.timestamp(), "2025-02-06 11:30:04");
    assert_eq!(decoded.price(), 1.2);
    assert_eq!(decoded.greeks_or_default().delta, 0.25);
    assert_eq!(decoded.underlying_price, Some(598.75));
}

#[test]
fn canonical_position_serialization_keeps_single_canonical_shape_in_bottom_library() {
    let position = sample_position(
        contract_at(580.0, OptionRight::Put),
        -2,
        Some(1.45),
        "shortput",
        Some(sample_snapshot(Some(1.4), Some(1.5))),
    );

    let encoded = serde_json::to_value(&position).unwrap();
    assert_eq!(
        encoded,
        json!({
            "contract": "SPY250321P00580000",
            "snapshot": {
                "as_of": "2025-02-06 11:30:04",
                "contract": {
                    "underlying_symbol": "SPY",
                    "expiration_date": "2025-03-21",
                    "strike": 600.0,
                    "option_right": "call",
                    "occ_symbol": "SPY250321C00600000"
                },
                "quote": {
                    "bid": 1.4,
                    "ask": 1.5,
                    "mark": 1.45,
                    "last": null
                },
                "greeks": null,
                "implied_volatility": null,
                "underlying_price": null
            },
            "qty": -2,
            "avg_cost": "1.45",
            "leg_type": "shortput"
        })
    );

    let decoded: OptionPosition = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.occ_symbol(), "SPY250321P00580000");
    assert_eq!(decoded.qty(), -2);
    assert_eq!(decoded.leg_type(), "shortput");
    assert_eq!(decoded.avg_cost(), 1.45);
}

#[test]
fn canonical_option_types_keep_generated_ts_shape_in_bottom_library() {
    let right_export = OptionRight::export_to_string().unwrap();
    assert!(right_export.contains("type OptionRight"));

    let snapshot_export = OptionSnapshot::export_to_string().unwrap();
    assert!(snapshot_export.contains("type OptionSnapshot"));
    assert!(snapshot_export.contains("as_of: string"));
    assert!(snapshot_export.contains("contract: OptionContract"));
    assert!(snapshot_export.contains("quote: OptionQuote"));
    assert!(snapshot_export.contains("implied_volatility: number | null"));

    let position_export = OptionPosition::export_to_string().unwrap();
    assert!(position_export.contains("type OptionPosition"));
    assert!(position_export.contains("contract: string"));
    assert!(position_export.contains("snapshot: OptionSnapshot"));
    assert!(position_export.contains("avg_cost: string"));
    assert!(position_export.contains("leg_type: string"));

    let contract_export = OptionContract::export_to_string().unwrap();
    assert!(contract_export.contains("type OptionContract"));
    assert!(contract_export.contains("underlying_symbol: string"));
    assert!(contract_export.contains("expiration_date: string"));

    let quote_export = OptionQuote::export_to_string().unwrap();
    assert!(quote_export.contains("type OptionQuote"));
    assert!(quote_export.contains("bid: number | null"));
    assert!(quote_export.contains("mark: number | null"));

    let chain_export = alpaca_option::OptionChain::export_to_string().unwrap();
    assert!(chain_export.contains("type OptionChain"));
    assert!(chain_export.contains("underlying_symbol: string"));
    assert!(chain_export.contains("as_of: string"));

    let record_export = OptionChainRecord::export_to_string().unwrap();
    assert!(record_export.contains("type OptionChainRecord"));
    assert!(record_export.contains("occ_symbol: string"));
    assert!(record_export.contains("option_right: OptionRight"));
}

#[test]
fn liquidity_models_stay_in_bottom_library() {
    let data = LiquidityOptionData::from_snapshot(
        &OptionSnapshot {
            as_of: "2025-02-06 11:30:04".to_string(),
            contract: contract_at(580.0, OptionRight::Put),
            quote: OptionQuote {
                bid: Some(2.15),
                ask: Some(2.35),
                mark: Some(2.25),
                last: Some(2.25),
            },
            greeks: Some(Greeks {
                delta: -0.3,
                gamma: 0.02,
                vega: 0.09,
                theta: -0.03,
                rho: -0.01,
            }),
            implied_volatility: Some(0.28),
            underlying_price: Some(600.0),
        },
        43,
    )
    .unwrap();

    assert_eq!(data.occ_symbol, "SPY250321P00580000");
    assert_eq!(data.option_right, "put");
    assert_eq!(data.expiration_date, "2025-03-21");
    assert_eq!(data.dte, 43);
    assert_eq!(data.mark, 2.25);
    assert_eq!(data.implied_volatility, 0.28);

    let stats = LiquidityStats::from_options(std::slice::from_ref(&data));
    assert_eq!(stats.total_count, 1);
    assert_eq!(stats.dte_range, (43, 43));

    let payload = LiquidityData {
        underlying_symbol: "SPY".to_string(),
        as_of: "2025-02-06 11:30:04".to_string(),
        underlying_price: 600.0,
        options: vec![data],
        stats,
    };
    assert_eq!(payload.options.len(), 1);

    let export = LiquidityData::export_to_string().unwrap();
    assert!(export.contains("type LiquidityData"));
    assert!(export.contains("underlying_symbol: string"));
    assert!(export.contains("as_of: string"));
    assert!(export.contains("underlying_price: number"));
}

#[test]
fn display_format_strike_keeps_expected_frontend_shape() {
    assert_eq!(display::format_strike(600.0), "600");
    assert_eq!(display::format_strike(600.5), "600.5");
    assert_eq!(display::format_strike(600.125), "600.125");
    assert_eq!(display::format_strike(600.120), "600.12");
}

#[test]
fn display_contract_display_keeps_contract_rendering_fields_in_core() {
    let put_contract = contract_at(580.0, OptionRight::Put);
    assert_eq!(
        display::contract_display(&put_contract, Some("yy-mm-dd")),
        ContractDisplay {
            strike: "580".to_string(),
            expiration: "25-03-21".to_string(),
            compact: "580P@25-03-21".to_string(),
            option_right_code: OptionRightCode::P,
        }
    );
}

#[test]
fn display_compact_contract_keeps_contract_rendering_in_core() {
    let put_contract = contract_at(580.0, OptionRight::Put);
    assert_eq!(display::compact_contract(&put_contract, None), "580P@03-21");

    let call_contract = sample_contract();
    assert_eq!(
        display::compact_contract(&call_contract, Some("yy-mm-dd")),
        "600C@25-03-21"
    );
}

#[test]
fn display_position_strike_and_analysis_position_otm_percent_keep_position_parsing_in_core() {
    let position = sample_position(
        contract_at(580.0, OptionRight::Put),
        -1,
        None,
        "shortput",
        None,
    );

    assert_eq!(display::position_strike(&position), "580");
    assert_eq!(
        analysis::position_otm_percent(598.0, &position).unwrap(),
        (598.0 - 580.0) / 598.0 * 100.0
    );
}

#[test]
fn analysis_annualized_premium_yield_days_keeps_dte_based_return_semantics_in_core() {
    assert_eq!(
        (analysis::annualized_premium_yield_days(2.0, 100.0, 14).unwrap() * 1_000_000.0).round()
            / 1_000_000.0,
        ((2.0_f64 / 100.0_f64 / (14.0_f64 / 365.0_f64)) * 1_000_000.0).round() / 1_000_000.0
    );
    assert_error_code(
        analysis::annualized_premium_yield_days(2.0, 100.0, 0).unwrap_err(),
        "invalid_analysis_input",
    );
}

#[test]
fn snapshot_helpers_keep_contract_and_quote_semantics_in_core() {
    let canonical = sample_snapshot(Some(1.1), Some(1.3));

    assert_eq!(
        snapshot::contract(&canonical).unwrap().occ_symbol,
        "SPY250321C00600000"
    );
    assert_eq!(snapshot::spread(&canonical), 0.2);
    assert_eq!(
        (snapshot::spread_pct(&canonical) * 1_000_000.0).round() / 1_000_000.0,
        ((0.2_f64 / 1.2_f64) * 1_000_000.0).round() / 1_000_000.0
    );
    assert!(snapshot::is_valid(&canonical));
    assert!(snapshot::liquidity(&canonical).is_some());

    let invalid = OptionSnapshot {
        as_of: String::new(),
        contract: OptionContract {
            underlying_symbol: "SPY".to_string(),
            expiration_date: "bad-date".to_string(),
            strike: 600.0,
            option_right: OptionRight::Call,
            occ_symbol: "SPY250332C00600000".to_string(),
        },
        quote: OptionQuote {
            bid: Some(1.1),
            ask: Some(1.3),
            mark: Some(1.2),
            last: Some(1.2),
        },
        greeks: None,
        implied_volatility: None,
        underlying_price: None,
    };
    assert!(!snapshot::is_valid(&invalid));
    assert_eq!(snapshot::liquidity(&invalid), None);
}

#[test]
fn snapshot_helpers_accept_custom_snapshot_like_inputs() {
    struct LegacySnapshot {
        contract: String,
        as_of: String,
        bid: f64,
        ask: f64,
        mark: f64,
        delta: f64,
    }

    impl snapshot::SnapshotLike for LegacySnapshot {
        fn canonical_contract(&self) -> Option<OptionContract> {
            contract::parse_occ_symbol(&self.contract)
        }

        fn as_of(&self) -> &str {
            &self.as_of
        }

        fn bid(&self) -> Option<f64> {
            Some(self.bid)
        }

        fn ask(&self) -> Option<f64> {
            Some(self.ask)
        }

        fn mark(&self) -> Option<f64> {
            Some(self.mark)
        }

        fn last(&self) -> Option<f64> {
            None
        }

        fn delta(&self) -> Option<f64> {
            Some(self.delta)
        }
    }

    let snapshot = LegacySnapshot {
        contract: "SPY250321P00580000".to_string(),
        as_of: "2025-02-06 11:30:04".to_string(),
        bid: 1.1,
        ask: 1.3,
        mark: 1.2,
        delta: -0.25,
    };

    assert_eq!(
        snapshot::contract(&snapshot).unwrap().occ_symbol,
        "SPY250321P00580000"
    );
    assert_eq!(snapshot::spread(&snapshot), 0.2);
    assert_eq!(
        (snapshot::spread_pct(&snapshot) * 1_000_000.0).round() / 1_000_000.0,
        ((0.2_f64 / 1.2_f64) * 1_000_000.0).round() / 1_000_000.0
    );
    assert_eq!(snapshot::liquidity(&snapshot), Some(true));
}

#[test]
fn contract_build_and_parse_absorb_invalid_occ_inputs_directly() {
    assert_eq!(
        contract::build_occ_symbol("SPY", "2025-03-21", 600.1254, "call"),
        Some("SPY250321C00600125".to_string())
    );

    assert_eq!(contract::parse_occ_symbol("SPY250321X00600000"), None);
    assert_eq!(contract::parse_occ_symbol("SPY250232C00600000"), None);
    assert_eq!(contract::parse_occ_symbol("SPY250321C00600A00"), None);
    assert_eq!(
        contract::build_occ_symbol("BRK.B-", "2025-03-21", 600.0, "call"),
        None
    );
    let position = sample_position(
        contract_at(580.0, OptionRight::Put),
        -1,
        None,
        "shortput",
        None,
    );
    assert_eq!(
        contract::canonical_contract(&position).map(|contract| contract.occ_symbol),
        Some("SPY250321P00580000".to_string())
    );
}

#[test]
fn optionstrat_helpers_cover_query_hash_optional_premium_and_underlying_mismatch() {
    let parsed = url::parse_optionstrat_url(
        "https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23?ref=abc#frag",
    )
    .unwrap();
    assert_eq!(parsed.underlying_display_symbol, "BRK.B");
    assert_eq!(parsed.leg_fragments, vec![".BRKB250620P480x1@1.23"]);

    let parsed_double_calendar = url::parse_optionstrat_url(
        "https://optionstrat.com/build/double-calendar/AVGO/.AVGO260821C430@31.67,-.AVGO260501C430@2.92,.AVGO260821P370@28.03,-.AVGO260501P370@3.24",
    )
    .unwrap();
    assert_eq!(parsed_double_calendar.underlying_display_symbol, "AVGO");
    assert_eq!(
        parsed_double_calendar.leg_fragments,
        vec![
            ".AVGO260821C430@31.67",
            "-.AVGO260501C430@2.92",
            ".AVGO260821P370@28.03",
            "-.AVGO260501P370@3.24",
        ]
    );

    let legs =
        url::parse_optionstrat_leg_fragments("BRK.B", &[".BRKB250620P480x1".to_string()]).unwrap();
    assert_eq!(legs.len(), 1);
    assert_eq!(legs[0].premium_per_contract, None);
    assert_eq!(legs[0].ratio_quantity, 1);

    assert_error_code(
        url::parse_optionstrat_leg_fragments("SPY", &[".QQQ250620P480x1@1.23".to_string()])
            .unwrap_err(),
        "invalid_optionstrat_leg_fragment",
    );
    assert_eq!(
        url::build_optionstrat_leg_fragment(&alpaca_option::OptionStratLegInput {
            occ_symbol: "SPY250321C00600000".to_string(),
            quantity: 0,
            premium_per_contract: Some(1.23),
            ..Default::default()
        }),
        None
    );
    assert_eq!(
        url::build_optionstrat_leg_fragment(&alpaca_option::OptionStratLegInput {
            occ_symbol: "SPY250321P00580000".to_string(),
            quantity: -1,
            premium_per_contract: Some(2.45),
            ..Default::default()
        }),
        Some("-.SPY250321P580x1@2.45".to_string())
    );
    assert_eq!(
        url::build_optionstrat_leg_fragment(&alpaca_option::OptionStratLegInput {
            occ_symbol: "SPY250321P00580000".to_string(),
            quantity: -1,
            premium_per_contract: None,
            ..Default::default()
        }),
        Some("-.SPY250321P580x1".to_string())
    );
    assert_eq!(
        url::build_optionstrat_stock_fragment(&alpaca_option::OptionStratStockInput {
            underlying_symbol: "BRK.B".to_string(),
            quantity: 100,
            cost_per_share: 512.34,
        }),
        Some("BRKBx100@512.34".to_string())
    );
    assert_eq!(
        url::build_optionstrat_stock_fragment(&alpaca_option::OptionStratStockInput {
            underlying_symbol: "BRK.B".to_string(),
            quantity: 0,
            cost_per_share: 512.34,
        }),
        None
    );
    assert_eq!(
        url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
            underlying_display_symbol: "BRK.B".to_string(),
            legs: vec![alpaca_option::OptionStratLegInput {
                occ_symbol: "BRKB250620P00480000".to_string(),
                quantity: -2,
                premium_per_contract: Some(12.34),
                ..Default::default()
            }],
            stocks: Vec::new(),
        }),
        Some("https://optionstrat.com/build/custom/BRK%2FB/-.BRKB250620P480x2@12.34".to_string())
    );
    assert_eq!(
        url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
            underlying_display_symbol: "SPY".to_string(),
            legs: vec![alpaca_option::OptionStratLegInput {
                occ_symbol: String::new(),
                underlying_symbol: Some("SPY".to_string()),
                expiration_date: Some("2025-03-21".to_string()),
                strike: Some(580.0),
                option_right: Some("put".to_string()),
                quantity: -1,
                premium_per_contract: Some(2.45),
                ..Default::default()
            }],
            stocks: Vec::new(),
        }),
        Some("https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45".to_string())
    );
    assert_eq!(
        url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
            underlying_display_symbol: "SPY".to_string(),
            legs: vec![alpaca_option::OptionStratLegInput {
                occ_symbol: "SPY250321P00580000".to_string(),
                quantity: -1,
                premium_per_contract: Some(2.45),
                ..Default::default()
            }],
            stocks: vec![alpaca_option::OptionStratStockInput {
                underlying_symbol: "SPY".to_string(),
                quantity: 100,
                cost_per_share: 530.12,
            }],
        }),
        Some(
            "https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,SPYx100@530.12"
                .to_string()
        )
    );
    assert_eq!(
        url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
            underlying_display_symbol: "SPY".to_string(),
            legs: Vec::new(),
            stocks: vec![alpaca_option::OptionStratStockInput {
                underlying_symbol: "SPY".to_string(),
                quantity: 100,
                cost_per_share: 530.12,
            }],
        }),
        Some("https://optionstrat.com/build/custom/SPY/SPYx100@530.12".to_string())
    );
    assert_eq!(
        url::build_optionstrat_url(&alpaca_option::OptionStratUrlInput {
            underlying_display_symbol: "SPY".to_string(),
            legs: Vec::new(),
            stocks: Vec::new(),
        }),
        None
    );
    assert_eq!(
        url::merge_optionstrat_urls(
            &[
                None,
                Some("bad-url".to_string()),
                Some("https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45".to_string()),
                Some(
                    "https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23"
                        .to_string()
                ),
                Some(
                    "https://optionstrat.com/build/custom/SPY/.SPY250321C600x2?ref=abc".to_string()
                ),
            ],
            Some("SPY"),
        ),
        Some(
            "https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,.SPY250321C600x2"
                .to_string()
        )
    );
    assert_eq!(
        url::merge_optionstrat_urls(
            &[
                Some("bad-url".to_string()),
                Some(
                    "https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23#frag"
                        .to_string()
                ),
            ],
            None,
        ),
        Some("https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23".to_string())
    );
    assert_eq!(
        url::merge_optionstrat_urls(&[Some("bad-url".to_string()), None], Some("SPY")),
        None
    );
}

#[test]
fn chain_helpers_absorb_snapshot_lookup_and_expiration_extraction_directly() {
    let snapshots = vec![
        alpaca_option::OptionSnapshot {
            as_of: "2025-02-06 11:30:04".to_string(),
            contract: contract::parse_occ_symbol("SPY250321P00580000").unwrap(),
            quote: OptionQuote {
                bid: Some(1.1),
                ask: Some(1.3),
                mark: Some(1.2),
                last: Some(1.2),
            },
            greeks: None,
            implied_volatility: None,
            underlying_price: Some(598.0),
        },
        alpaca_option::OptionSnapshot {
            as_of: "2025-02-06 11:30:04".to_string(),
            contract: contract::parse_occ_symbol("SPY250328P00580000").unwrap(),
            quote: OptionQuote {
                bid: Some(1.4),
                ask: Some(1.6),
                mark: Some(1.5),
                last: Some(1.5),
            },
            greeks: None,
            implied_volatility: None,
            underlying_price: Some(598.0),
        },
        alpaca_option::OptionSnapshot {
            as_of: "2025-02-06 11:30:04".to_string(),
            contract: contract::parse_occ_symbol("SPY250328C00600000").unwrap(),
            quote: OptionQuote {
                bid: Some(2.4),
                ask: Some(2.8),
                mark: Some(2.6),
                last: Some(2.6),
            },
            greeks: None,
            implied_volatility: None,
            underlying_price: Some(598.0),
        },
    ];
    let option_chain = alpaca_option::OptionChain {
        underlying_symbol: "SPY".to_string(),
        as_of: "2025-02-06 11:30:04".to_string(),
        snapshots,
    };

    let snapshot = chain::find_snapshot(
        &option_chain,
        chain::SnapshotFilter {
            option_right: Some("put"),
            expiration_date: Some("2025-03-28"),
            strike: Some(580.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(snapshot.contract.occ_symbol, "SPY250328P00580000");

    let filtered = chain::list_snapshots(
        &option_chain,
        chain::SnapshotFilter {
            expiration_date: Some("2025-03-28"),
            ..Default::default()
        },
    );
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].contract.occ_symbol, "SPY250328P00580000");
    assert_eq!(filtered[1].contract.occ_symbol, "SPY250328C00600000");

    let expirations = chain::expiration_dates(
        &option_chain,
        Some("put"),
        Some(30),
        Some(60),
        Some("2025-02-20 09:30:00"),
    )
    .unwrap();
    assert_eq!(
        expirations,
        vec![chain::ExpirationDate {
            expiration_date: "2025-03-28".to_string(),
            calendar_days: 36,
        }]
    );
}

#[test]
fn analysis_assignment_risk_exposes_stable_thresholds_for_app_side_rendering() {
    assert_eq!(analysis::assignment_risk(-0.01).unwrap().as_str(), "danger");
    assert_eq!(
        analysis::assignment_risk(0.03).unwrap().as_str(),
        "critical"
    );
    assert_eq!(analysis::assignment_risk(0.08).unwrap().as_str(), "high");
    assert_eq!(analysis::assignment_risk(0.2).unwrap().as_str(), "medium");
    assert_eq!(analysis::assignment_risk(0.5).unwrap().as_str(), "low");
    assert_eq!(analysis::assignment_risk(1.2).unwrap().as_str(), "safe");
}

#[test]
fn analysis_short_extrinsic_amount_handles_canonical_short_positions() {
    let positions = vec![
        sample_position(
            contract_at(100.0, OptionRight::Call),
            -1,
            None,
            "shortcall",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: contract_at(100.0, OptionRight::Call),
                quote: OptionQuote {
                    bid: Some(5.1),
                    ask: Some(5.4),
                    mark: Some(5.25),
                    last: Some(5.2),
                },
                greeks: None,
                implied_volatility: None,
                underlying_price: Some(105.0),
            }),
        ),
        sample_position(
            contract_at(95.0, OptionRight::Put),
            -2,
            None,
            "shortput",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: contract_at(95.0, OptionRight::Put),
                quote: OptionQuote {
                    bid: Some(0.5),
                    ask: Some(0.7),
                    mark: Some(0.6),
                    last: Some(0.61),
                },
                greeks: None,
                implied_volatility: None,
                underlying_price: Some(105.0),
            }),
        ),
        sample_position(
            contract_at(110.0, OptionRight::Call),
            1,
            None,
            "longcall",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: contract_at(110.0, OptionRight::Call),
                quote: OptionQuote {
                    bid: Some(0.2),
                    ask: Some(0.3),
                    mark: Some(0.25),
                    last: Some(0.25),
                },
                greeks: None,
                implied_volatility: None,
                underlying_price: Some(105.0),
            }),
        ),
    ];

    assert_eq!(
        analysis::short_extrinsic_amount(105.0, &positions, Some(2)).unwrap(),
        Some(290.0)
    );

    let missing_price = vec![sample_position(
        contract_at(100.0, OptionRight::Call),
        -1,
        None,
        "shortcall",
        Some(OptionSnapshot {
            as_of: "2025-02-06 11:30:04".to_string(),
            contract: contract_at(100.0, OptionRight::Call),
            quote: OptionQuote {
                bid: None,
                ask: None,
                mark: None,
                last: None,
            },
            greeks: None,
            implied_volatility: None,
            underlying_price: Some(105.0),
        }),
    )];
    assert_eq!(
        analysis::short_extrinsic_amount(105.0, &missing_price, None).unwrap(),
        None
    );
}

#[test]
fn analysis_short_itm_positions_handle_canonical_short_positions() {
    let positions = vec![
        sample_position(
            contract_at(100.0, OptionRight::Call),
            -1,
            None,
            "shortcall",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: contract_at(100.0, OptionRight::Call),
                quote: OptionQuote {
                    bid: Some(5.1),
                    ask: Some(5.4),
                    mark: Some(5.25),
                    last: Some(5.2),
                },
                greeks: None,
                implied_volatility: None,
                underlying_price: Some(105.0),
            }),
        ),
        sample_position(
            contract_at(110.0, OptionRight::Put),
            -2,
            None,
            "shortput",
            None,
        ),
        sample_position(
            contract_at(110.0, OptionRight::Call),
            1,
            None,
            "longcall",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: contract_at(110.0, OptionRight::Call),
                quote: OptionQuote {
                    bid: Some(0.2),
                    ask: Some(0.3),
                    mark: Some(0.25),
                    last: Some(0.25),
                },
                greeks: None,
                implied_volatility: None,
                underlying_price: Some(105.0),
            }),
        ),
    ];

    let items = analysis::short_itm_positions(105.0, &positions).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].contract.occ_symbol, "SPY250321C00100000");
    assert_eq!(items[0].quantity, 1);
    assert_eq!(items[0].option_price, 5.25);
    assert_eq!(items[0].intrinsic, 5.0);
    assert_eq!(items[0].extrinsic, 0.25);
    assert_eq!(items[1].contract.occ_symbol, "SPY250321P00110000");
    assert_eq!(items[1].quantity, 2);
    assert_eq!(items[1].option_price, 0.0);
    assert_eq!(items[1].intrinsic, 5.0);
    assert_eq!(items[1].extrinsic, 0.0);
}

#[test]
fn pricing_contract_extrinsic_value_keeps_contract_semantics_in_core() {
    let contract = contract::parse_occ_symbol("SPY250321P00580000").unwrap();
    assert_eq!(
        pricing::contract_extrinsic_value(1.5, 590.0, &contract).unwrap(),
        1.5
    );
}

#[test]
fn pricing_extrinsic_value_floors_below_intrinsic_quotes_directly() {
    assert_eq!(
        pricing::extrinsic_value(4.5, 110.0, 100.0, "call").unwrap(),
        0.0
    );
    assert_eq!(
        pricing::extrinsic_value(0.75, 95.0, 100.0, "put").unwrap(),
        0.0
    );
    assert_eq!(
        pricing::extrinsic_value(2.5, 103.0, 100.0, "call").unwrap(),
        0.0
    );
}

#[test]
fn execution_quote_clamps_progress_and_handles_degenerate_ranges() {
    assert_eq!(
        execution_quote::limit_quote_by_progress(1.25, 1.45, -1.0).unwrap(),
        1.25
    );
    assert_eq!(
        execution_quote::limit_quote_by_progress(1.25, 1.45, 200.0).unwrap(),
        1.45
    );
    assert_eq!(
        execution_quote::limit_quote_by_progress(1.25, 1.45, 40.0).unwrap(),
        1.33
    );
    assert_eq!(
        execution_quote::progress_of_limit(1.25, 1.45, 2.0).unwrap(),
        1.0
    );
    assert_eq!(
        execution_quote::progress_of_limit(1.25, 1.45, 1.0).unwrap(),
        0.0
    );
    assert_eq!(
        execution_quote::progress_of_limit(1.25, 1.25, 1.25).unwrap(),
        0.5
    );

    let positions = vec![
        sample_position(
            sample_contract(),
            2,
            None,
            "longcall",
            Some(sample_snapshot(Some(1.10), Some(1.30))),
        ),
        sample_position(
            sample_contract(),
            -1,
            None,
            "shortcall",
            Some(sample_snapshot(Some(0.60), Some(0.80))),
        ),
    ];
    let summary = execution_quote::best_worst(positions.as_slice(), None).unwrap();
    assert_eq!(summary.structure_quantity, 1);
    assert_eq!(summary.per_structure.best_price, 1.4);
    assert_eq!(summary.per_structure.worst_price, 2.0);
    assert_eq!(summary.dollars.best_price, 140.0);
    assert_eq!(summary.dollars.worst_price, 200.0);
}

#[test]
fn execution_quote_quote_and_limit_price_keep_boundary_behavior_simple() {
    let snapshot = OptionSnapshot {
        as_of: "2025-02-06 11:30:04".to_string(),
        contract: sample_contract(),
        quote: OptionQuote {
            bid: Some(1.1),
            ask: Some(1.3),
            mark: None,
            last: None,
        },
        greeks: None,
        implied_volatility: None,
        underlying_price: None,
    };

    let normalized = execution_quote::quote(&snapshot);
    assert_eq!(normalized.bid, Some(1.1));
    assert_eq!(normalized.ask, Some(1.3));
    assert_eq!(normalized.mark, Some(1.2));
    assert_eq!(normalized.last, Some(1.2));

    assert_eq!(execution_quote::limit_price(Some(1.45)), 1.45);
    assert_eq!(execution_quote::limit_price(None), 0.0);
    assert_eq!(execution_quote::limit_price(Some(f64::NAN)), 0.0);
}

#[test]
fn execution_quote_roll_request_absorbs_target_contracts_and_explicit_fields_directly() {
    let from_target = execution_quote::roll_request(
        "SPY250321P00580000",
        Some("SPY260417P00570000"),
        None,
        None,
        Some("ShortPut"),
        Some(2),
    )
    .unwrap();
    assert_eq!(from_target.current_contract, "SPY250321P00580000");
    assert_eq!(from_target.leg_type, Some("shortput".to_string()));
    assert_eq!(from_target.qty, 2);
    assert_eq!(from_target.new_strike, Some(570.0));
    assert_eq!(from_target.new_expiration, "2026-04-17");

    let from_fields = execution_quote::roll_request(
        "SPY250321C00600000",
        None,
        Some(605.0),
        Some("2026-04-25"),
        None,
        Some(0),
    )
    .unwrap();
    assert_eq!(from_fields.current_contract, "SPY250321C00600000");
    assert_eq!(from_fields.leg_type, None);
    assert_eq!(from_fields.qty, 1);
    assert_eq!(from_fields.new_strike, Some(605.0));
    assert_eq!(from_fields.new_expiration, "2026-04-25");

    assert_eq!(
        execution_quote::roll_request(
            "SPY250321C00600000",
            Some("bad-contract"),
            None,
            None,
            None,
            None
        ),
        None
    );
    assert_eq!(
        execution_quote::roll_request("", None, Some(605.0), Some("2026-04-25"), None, None),
        None
    );
}

#[test]
fn execution_quote_leg_type_absorbs_legacy_order_leg_payloads_directly() {
    assert_eq!(
        alpaca_option::OrderSide::from_str(" Buy ")
            .unwrap()
            .as_str(),
        "buy"
    );
    assert!(
        alpaca_option::PositionIntent::from_str("sell_to_close")
            .unwrap()
            .is_close()
    );
    assert_eq!(
        execution_quote::leg_type("SPY250321P00580000", "buy", "buy_to_close", None),
        Some("shortput".to_string())
    );
    assert_eq!(
        execution_quote::leg_type("SPY250321C00600000", "sell", "sell_to_open", None),
        Some("shortcall".to_string())
    );
    assert_eq!(
        execution_quote::leg_type("SPY250321C00600000", "buy", "buy_to_open", Some("LongCall"),),
        Some("longcall".to_string())
    );
    assert_eq!(
        execution_quote::leg_type(
            "SPY250321C00600000",
            "buy",
            "buy_to_open",
            Some("longcall_low"),
        ),
        Some("longcall".to_string())
    );
    assert_eq!(
        execution_quote::leg_type(
            "SPY250321P00580000",
            "sell",
            "sell_to_open",
            Some("diagonal_shortput"),
        ),
        Some("shortput".to_string())
    );
    assert_eq!(
        execution_quote::leg_type("bad-contract", "buy", "buy_to_open", None),
        None
    );
}

#[test]
fn execution_quote_order_legs_and_roll_legs_keep_execution_mapping_in_one_place() {
    let long_call_contract = contract_at(100.0, OptionRight::Call);
    let short_put_contract = contract_at(95.0, OptionRight::Put);
    let positions = vec![
        sample_position(
            long_call_contract.clone(),
            2,
            None,
            "longcall",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: long_call_contract.clone(),
                quote: OptionQuote {
                    bid: Some(1.1),
                    ask: Some(1.3),
                    mark: Some(1.2),
                    last: Some(1.2),
                },
                greeks: None,
                implied_volatility: Some(0.25),
                underlying_price: Some(600.0),
            }),
        ),
        sample_position(
            short_put_contract.clone(),
            -1,
            None,
            "shortput",
            Some(OptionSnapshot {
                as_of: "2025-02-06 11:30:04".to_string(),
                contract: short_put_contract.clone(),
                quote: OptionQuote {
                    bid: Some(2.15),
                    ask: Some(2.35),
                    mark: Some(2.25),
                    last: Some(2.25),
                },
                greeks: None,
                implied_volatility: Some(0.28),
                underlying_price: Some(600.0),
            }),
        ),
    ];

    let close_legs = execution_quote::order_legs(&positions, "close", None, None).unwrap();
    assert_eq!(close_legs.len(), 2);
    assert_eq!(close_legs[0].symbol, "SPY250321C00100000");
    assert_eq!(close_legs[0].ratio_qty, "2");
    assert_eq!(close_legs[0].side.as_str(), "sell");
    assert_eq!(close_legs[0].position_intent.as_str(), "sell_to_close");
    assert_eq!(close_legs[0].leg_type, "longcall");
    assert_eq!(
        close_legs[0]
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.contract.as_str()),
        Some("SPY250321C00100000")
    );
    assert_eq!(
        close_legs[0]
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.bid.as_str()),
        Some("1.1")
    );

    let include_leg_types = vec!["shortput".to_string()];
    let open_legs =
        execution_quote::order_legs(&positions, "open", Some(&include_leg_types), None).unwrap();
    assert_eq!(open_legs.len(), 1);
    assert_eq!(open_legs[0].symbol, "SPY250321P00095000");
    assert_eq!(open_legs[0].ratio_qty, "1");
    assert_eq!(open_legs[0].side.as_str(), "sell");
    assert_eq!(open_legs[0].position_intent.as_str(), "sell_to_open");
    assert_eq!(open_legs[0].leg_type, "shortput");

    let next_snapshot = execution_quote::ExecutionSnapshot {
        contract: "SPY250328P00090000".to_string(),
        timestamp: "2025-02-06 11:31:00".to_string(),
        bid: "1.75".to_string(),
        ask: "1.95".to_string(),
        price: "1.85".to_string(),
        greeks: execution_quote::Greeks {
            delta: -0.24,
            gamma: 0.02,
            vega: 0.08,
            theta: -0.02,
            rho: -0.01,
        },
        iv: 0.24,
    };
    let snapshots = std::collections::HashMap::from([("shortput".to_string(), next_snapshot)]);
    let selections = vec![execution_quote::RollLegSelection {
        leg_type: "shortput".to_string(),
        quantity: Some(1),
    }];

    let roll_legs = execution_quote::roll_legs(&positions, &snapshots, &selections).unwrap();
    assert_eq!(roll_legs.len(), 2);
    assert_eq!(roll_legs[0].symbol, "SPY250321P00095000");
    assert_eq!(roll_legs[0].side.as_str(), "buy");
    assert_eq!(roll_legs[0].position_intent.as_str(), "buy_to_close");
    assert_eq!(roll_legs[1].symbol, "SPY250328P00090000");
    assert_eq!(roll_legs[1].side.as_str(), "sell");
    assert_eq!(roll_legs[1].position_intent.as_str(), "sell_to_open");
    assert_eq!(roll_legs[1].leg_type, "shortput");
}

#[test]
fn execution_quote_leg_builds_a_single_execution_leg_from_direct_quote_inputs() {
    let built = execution_quote::leg(execution_quote::ExecutionLegInput {
        action: alpaca_option::ExecutionAction::Open,
        leg_type: "longcall_low".to_string(),
        contract: "SPY250321C00600000".to_string(),
        quantity: Some(1),
        snapshot: None,
        timestamp: Some("2025-02-06 11:30:04".to_string()),
        bid: None,
        ask: None,
        price: Some(1.2),
        spread_percent: Some(0.1),
        greeks: Some(execution_quote::GreeksInput {
            delta: Some(0.5),
            gamma: None,
            vega: None,
            theta: Some(-0.03),
            rho: None,
        }),
        iv: Some(0.25),
    })
    .unwrap();

    assert_eq!(built.symbol, "SPY250321C00600000");
    assert_eq!(built.ratio_qty, "1");
    assert_eq!(built.side.as_str(), "buy");
    assert_eq!(built.position_intent.as_str(), "buy_to_open");
    assert_eq!(built.leg_type, "longcall");
    assert_eq!(
        built
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.bid.as_str()),
        Some("1.14")
    );
    assert_eq!(
        built
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.ask.as_str()),
        Some("1.26")
    );

    assert!(
        execution_quote::leg(execution_quote::ExecutionLegInput {
            action: alpaca_option::ExecutionAction::Open,
            leg_type: "longcall".to_string(),
            contract: "bad-contract".to_string(),
            quantity: Some(1),
            snapshot: None,
            timestamp: Some("2025-02-06 11:30:04".to_string()),
            bid: None,
            ask: None,
            price: Some(1.2),
            spread_percent: Some(0.1),
            greeks: None,
            iv: None,
        })
        .is_none()
    );
    assert!(
        execution_quote::leg(execution_quote::ExecutionLegInput {
            action: alpaca_option::ExecutionAction::Open,
            leg_type: "longcall".to_string(),
            contract: "SPY250321P00580000".to_string(),
            quantity: Some(1),
            snapshot: None,
            timestamp: Some("2025-02-06 11:30:04".to_string()),
            bid: Some(1.1),
            ask: Some(1.3),
            price: None,
            spread_percent: None,
            greeks: None,
            iv: None,
        })
        .is_none()
    );
}

#[test]
fn execution_quote_leg_preserves_explicit_quantity() {
    let built = execution_quote::leg(execution_quote::ExecutionLegInput {
        action: alpaca_option::ExecutionAction::Close,
        leg_type: "shortcall".to_string(),
        contract: "SPY250321C00600000".to_string(),
        quantity: Some(2),
        snapshot: None,
        timestamp: Some("2025-02-06 11:30:04".to_string()),
        bid: Some(1.1),
        ask: Some(1.3),
        price: None,
        spread_percent: None,
        greeks: None,
        iv: None,
    })
    .unwrap();

    assert_eq!(built.ratio_qty, "2");
    assert_eq!(built.side.as_str(), "buy");
    assert_eq!(built.position_intent.as_str(), "buy_to_close");
}

#[test]
fn payoff_and_probability_boundary_cases_are_explicit() {
    assert_eq!(payoff::strategy_payoff_at_expiry(&[], 100.0).unwrap(), 0.0);
    assert_eq!(payoff::break_even_points(&[]).unwrap(), Vec::<f64>::new());

    let short_call = PayoffLegInput::new("call", "short", 100.0, 2.0, 1).unwrap();
    assert_eq!(
        payoff::strategy_payoff_at_expiry(&[short_call], 90.0).unwrap(),
        2.0
    );

    assert_error_code(
        payoff::strategy_payoff_at_expiry(&[], -1.0).unwrap_err(),
        "invalid_payoff_input",
    );
    assert_error_code(
        probability::expiry_probability_in_range(100.0, 105.0, 95.0, 0.1, 0.045, 0.0, 0.2)
            .unwrap_err(),
        "invalid_probability_input",
    );
}

#[test]
fn numeric_solver_surfaces_bracketing_and_convergence_errors() {
    assert_error_code(
        numeric::brent_solve(1.0, 2.0, |x| x * x + 1.0, None, None).unwrap_err(),
        "root_not_bracketed",
    );
    assert_error_code(
        numeric::brent_solve(0.0, 2.0, |x| x * x - 2.0, Some(1e-20), Some(1)).unwrap_err(),
        "root_not_converged",
    );
}

#[test]
fn pricing_implied_volatility_uses_discounted_no_arbitrage_lower_bounds() {
    let implied_volatility = pricing::implied_volatility_from_price(
        &alpaca_option::BlackScholesImpliedVolatilityInput {
            target_price: 130.65325629560832,
            spot: 250.0,
            strike: 100.0,
            years: 10.0,
            rate: 0.03,
            dividend_yield: 0.02,
            option_right: OptionRight::Call,
            lower_bound: Some(0.000001),
            upper_bound: Some(5.0),
            tolerance: Some(1e-12),
            max_iterations: None,
        },
    )
    .unwrap();

    assert!((implied_volatility - 0.12).abs() <= 1e-10);
}

#[test]
fn advanced_math_kernels_surface_explicit_boundary_errors() {
    assert_error_code(
        math::american::discrete_dividend_price(100.0, 95.0, 1.0, 0.03, 0.25, "call", "spot", &[])
            .unwrap_err(),
        "invalid_math_input",
    );
    assert_error_code(
        math::barrier::price(
            125.0, 105.0, 125.0, 0.0, 0.5, 0.01, 0.02, 0.3, "put", "up_out",
        )
        .unwrap_err(),
        "invalid_math_input",
    );
    assert_error_code(
        math::geometric_asian::price(100.0, 100.0, 1.0, 0.03, 0.01, 0.25, "call", "discrete")
            .unwrap_err(),
        "unsupported_math_input",
    );
}

#[test]
fn option_right_from_str_absorbs_common_case_and_code_variants() {
    assert_eq!(OptionRight::from_str("call").unwrap(), OptionRight::Call);
    assert_eq!(OptionRight::from_str(" Put ").unwrap(), OptionRight::Put);
    assert_eq!(OptionRight::from_str("C").unwrap(), OptionRight::Call);
    assert_eq!(OptionRight::from_str("p").unwrap(), OptionRight::Put);
}
