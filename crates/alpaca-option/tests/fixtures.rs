use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use alpaca_option::{
    ExecutionLegInput, ExecutionSnapshot, OptionChain, OptionChainRecord, OptionContract,
    OptionPosition, OptionRight, OptionSnapshot, OptionStrategy, OptionStrategyInput,
    PayoffLegInput, QuotedLeg, RollLegSelection, StrategyBreakEvenInput, StrategyPnlInput,
    StrategyValuationPosition, analysis, contract, execution_quote,
    expiration_selection, math, numeric, payoff, pricing, probability, url,
};

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_cases(relative_path: &str) -> Vec<Value> {
    let content =
        fs::read_to_string(repo_root().join(relative_path)).expect("fixture should exist");
    serde_json::from_str::<Value>(&content)
        .expect("fixture json should parse")
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .expect("fixture cases should be array")
}

fn load_support_fixture_paths() -> Vec<String> {
    let content = fs::read_to_string(repo_root().join("fixtures/catalog.json"))
        .expect("catalog should exist");
    let catalog = serde_json::from_str::<Value>(&content).expect("catalog json should parse");

    catalog
        .get("support_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn load_layer_fixture_paths() -> Vec<String> {
    let content = fs::read_to_string(repo_root().join("fixtures/catalog.json"))
        .expect("catalog should exist");
    let catalog = serde_json::from_str::<Value>(&content).expect("catalog json should parse");

    let layer_paths = catalog
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|layer| layer.get("status").and_then(Value::as_str) == Some("integrated"))
        .flat_map(|layer| {
            layer
                .get("fixture_paths")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });

    layer_paths.collect()
}

fn fixture_requires_explicit_rate(api: &str) -> bool {
    matches!(
        api,
        "analysis.strike_for_target_delta"
            | "math.american.discrete_dividend_price"
            | "math.american_barone_adesi_whaley_price"
            | "math.american_bjerksund_stensland_1993_price"
            | "math.american_ju_quadratic_price"
            | "math.american_tree_price"
            | "math.bachelier_greeks"
            | "math.bachelier_implied_volatility_from_price"
            | "math.bachelier_price"
            | "math.barrier.price"
            | "math.black76_greeks"
            | "math.black76_implied_volatility_from_price"
            | "math.black76_price"
            | "math.geometric_asian.price"
            | "payoff.strategy_break_even_points"
            | "payoff.strategy_pnl"
            | "payoff.option_strategy_curve"
            | "payoff.option_strategy_model_greeks"
            | "pricing.black_scholes_put_call_parity"
            | "pricing.greeks_black_scholes"
            | "pricing.implied_volatility_from_price"
            | "pricing.price_black_scholes"
            | "probability.expiry_probability_in_range"
    )
}

fn assert_rate_dependent_cases_have_explicit_rate(relative_path: &str) {
    for case in load_cases(relative_path) {
        let api = case
            .get("api")
            .and_then(Value::as_str)
            .expect("fixture case should include api");

        if !fixture_requires_explicit_rate(api) {
            continue;
        }

        let id = case
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<missing id>");
        let rate = case
            .get("input")
            .and_then(|input| input.get("rate"))
            .and_then(Value::as_f64);
        assert!(
            rate.is_some_and(f64::is_finite),
            "{relative_path}:{id} ({api}) must include finite input.rate"
        );
    }
}

fn unwrap_expected(expected: &Value) -> Value {
    expected
        .get("value")
        .cloned()
        .unwrap_or_else(|| expected.clone())
}

fn fixture_avg_cost_value(input: &Value) -> Value {
    input
        .get("avg_cost")
        .or_else(|| input.get("avg_entry_price"))
        .cloned()
        .filter(|value| !value.is_null())
        .unwrap_or_else(|| Value::from(0.0))
}

fn fixture_snapshot(input: &Value) -> OptionSnapshot {
    input
        .get("snapshot")
        .cloned()
        .filter(|value| !value.is_null())
        .and_then(|value| serde_json::from_value::<OptionSnapshot>(value).ok())
        .unwrap_or_default()
}

fn fixture_contract(input: &Value, snapshot: &OptionSnapshot) -> OptionContract {
    let Some(contract_value) = input.get("contract") else {
        return snapshot.contract.clone();
    };

    if let Ok(contract) = serde_json::from_value::<OptionContract>(contract_value.clone()) {
        return contract;
    }

    if let Some(occ_symbol) = contract_value.as_str() {
        return contract::parse_occ_symbol(occ_symbol).unwrap_or_else(|| OptionContract {
            occ_symbol: occ_symbol.to_string(),
            ..OptionContract::default()
        });
    }

    snapshot.contract.clone()
}

fn fixture_position_side(input: &Value, qty: i32, leg_type: Option<&str>) -> String {
    input
        .get("position_side")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .or_else(|| {
            leg_type.map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                if normalized.starts_with("short") {
                    "short".to_string()
                } else {
                    "long".to_string()
                }
            })
        })
        .unwrap_or_else(|| {
            if qty < 0 {
                "short".to_string()
            } else {
                "long".to_string()
            }
        })
}

fn fixture_position(input: &Value) -> OptionPosition {
    if let Ok(position) = serde_json::from_value::<OptionPosition>(input.clone()) {
        return position;
    }

    let snapshot = fixture_snapshot(input);
    let contract = fixture_contract(input, &snapshot);
    let raw_qty = input
        .get("qty")
        .or_else(|| input.get("quantity"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let provided_leg_type = input.get("leg_type").and_then(Value::as_str);
    let position_side = fixture_position_side(input, raw_qty, provided_leg_type);
    let qty = match position_side.as_str() {
        "short" => -raw_qty.abs(),
        "long" => raw_qty.abs(),
        _ => raw_qty,
    };
    let leg_type = provided_leg_type
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{}{}", position_side, contract.option_right.as_str()));

    serde_json::from_value::<OptionPosition>(serde_json::json!({
        "contract": contract.occ_symbol,
        "snapshot": if snapshot.is_empty() {
            Value::Null
        } else {
            serde_json::to_value(snapshot).unwrap()
        },
        "qty": qty,
        "avg_cost": fixture_avg_cost_value(input),
        "leg_type": leg_type,
    }))
    .unwrap()
}

fn fixture_positions(input: &Value) -> Vec<OptionPosition> {
    input
        .as_array()
        .unwrap()
        .iter()
        .map(fixture_position)
        .collect()
}

fn black_scholes_put_call_parity(input: &Value) -> Value {
    let spot = input.get("spot").unwrap().as_f64().unwrap();
    let strike = input.get("strike").unwrap().as_f64().unwrap();
    let years = input.get("years").unwrap().as_f64().unwrap();
    let rate = input.get("rate").unwrap().as_f64().unwrap();
    let dividend_yield = input.get("dividend_yield").unwrap().as_f64().unwrap();
    let volatility = input.get("volatility").unwrap().as_f64().unwrap();

    let call = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot,
        strike,
        years,
        rate,
        dividend_yield,
        volatility,
        option_right: OptionRight::Call,
    })
    .unwrap();
    let put = pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
        spot,
        strike,
        years,
        rate,
        dividend_yield,
        volatility,
        option_right: OptionRight::Put,
    })
    .unwrap();

    serde_json::json!({
        "call_minus_put": call - put,
        "discounted_forward_minus_strike": spot * (-dividend_yield * years).exp() - strike * (-rate * years).exp(),
    })
}

fn resolve_tolerance(
    tolerance: Option<f64>,
    field_tolerances: Option<&HashMap<String, f64>>,
    path: &[String],
) -> Option<f64> {
    let Some(field_tolerances) = field_tolerances else {
        return tolerance;
    };

    let full_path = path.join(".");
    if !full_path.is_empty() {
        if let Some(value) = field_tolerances.get(&full_path) {
            return Some(*value);
        }
    }

    if let Some(leaf) = path.last() {
        if let Some(value) = field_tolerances.get(leaf) {
            return Some(*value);
        }
    }

    tolerance
}

fn assert_with_tolerance(
    actual: &Value,
    expected: &Value,
    tolerance: Option<f64>,
    case_id: &str,
    field_tolerances: Option<&HashMap<String, f64>>,
    path: &[String],
) {
    match (actual, expected) {
        (Value::Number(actual_number), Value::Number(expected_number)) => {
            let actual_value = actual_number.as_f64().expect("actual number should be f64");
            let expected_value = expected_number
                .as_f64()
                .expect("expected number should be f64");
            if let Some(limit) = resolve_tolerance(tolerance, field_tolerances, path) {
                assert!(
                    (actual_value - expected_value).abs() <= limit,
                    "{case_id}: expected {expected_value}, got {actual_value}, tolerance {limit}"
                );
            } else {
                assert_eq!(actual, expected, "{case_id}");
            }
        }
        (Value::Array(actual_items), Value::Array(expected_items)) => {
            assert_eq!(actual_items.len(), expected_items.len(), "{case_id}");
            for (index, (actual_item, expected_item)) in
                actual_items.iter().zip(expected_items.iter()).enumerate()
            {
                let mut child_path = path.to_vec();
                child_path.push(index.to_string());
                assert_with_tolerance(
                    actual_item,
                    expected_item,
                    tolerance,
                    case_id,
                    field_tolerances,
                    &child_path,
                );
            }
        }
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            assert_eq!(actual_map.len(), expected_map.len(), "{case_id}");
            for (key, expected_value) in expected_map {
                let actual_value = actual_map
                    .get(key)
                    .unwrap_or_else(|| panic!("{case_id}: missing key {key}"));
                let mut child_path = path.to_vec();
                child_path.push(key.clone());
                assert_with_tolerance(
                    actual_value,
                    expected_value,
                    tolerance,
                    case_id,
                    field_tolerances,
                    &child_path,
                );
            }
        }
        _ => assert_eq!(actual, expected, "{case_id}"),
    }
}

fn run_case(case: &Value) -> Value {
    let api = case.get("api").and_then(Value::as_str).unwrap();
    let input = case.get("input").unwrap();

    match api {
        "contract.normalize_underlying_symbol" => serde_json::to_value(
            contract::normalize_underlying_symbol(input.get("symbol").unwrap().as_str().unwrap()),
        )
        .unwrap(),
        "contract.is_occ_symbol" => serde_json::to_value(contract::is_occ_symbol(
            input.get("occ_symbol").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "contract.parse_occ_symbol" => serde_json::to_value(contract::parse_occ_symbol(
            input.get("occ_symbol").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "contract.build_occ_symbol" => serde_json::to_value(contract::build_occ_symbol(
            input.get("underlying_symbol").unwrap().as_str().unwrap(),
            input.get("expiration_date").unwrap().as_str().unwrap(),
            input
                .get("strike")
                .and_then(Value::as_f64)
                .or_else(|| {
                    input
                        .get("strike")
                        .and_then(Value::as_str)
                        .and_then(|value| value.parse::<f64>().ok())
                })
                .unwrap_or(f64::NAN),
            input.get("option_right").unwrap().as_str().unwrap(),
        ))
        .unwrap(),
        "url.to_optionstrat_underlying_path" => serde_json::to_value(
            url::to_optionstrat_underlying_path(input.get("symbol").unwrap().as_str().unwrap()),
        )
        .unwrap(),
        "url.from_optionstrat_underlying_path" => serde_json::to_value(
            url::from_optionstrat_underlying_path(input.get("path").unwrap().as_str().unwrap()),
        )
        .unwrap(),
        "url.build_optionstrat_leg_fragment" => serde_json::to_value(
            url::build_optionstrat_leg_fragment(&alpaca_option::OptionStratLegInput {
                occ_symbol: input
                    .get("occ_symbol")
                    .and_then(Value::as_str)
                    .unwrap()
                    .to_string(),
                quantity: input.get("quantity").and_then(Value::as_i64).unwrap() as i32,
                premium_per_contract: input.get("premium_per_contract").and_then(Value::as_f64),
                ..Default::default()
            }),
        )
        .unwrap(),
        "url.build_optionstrat_stock_fragment" => serde_json::to_value(
            url::build_optionstrat_stock_fragment(&alpaca_option::OptionStratStockInput {
                underlying_symbol: input
                    .get("underlying_symbol")
                    .and_then(Value::as_str)
                    .unwrap()
                    .to_string(),
                quantity: input.get("quantity").and_then(Value::as_i64).unwrap() as i32,
                cost_per_share: input.get("cost_per_share").and_then(Value::as_f64).unwrap(),
            }),
        )
        .unwrap(),
        "url.build_optionstrat_url" => {
            let legs = input
                .get("legs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|leg| alpaca_option::OptionStratLegInput {
                    occ_symbol: leg
                        .get("occ_symbol")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string(),
                    quantity: leg.get("quantity").and_then(Value::as_i64).unwrap() as i32,
                    premium_per_contract: leg.get("premium_per_contract").and_then(Value::as_f64),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let stocks = input
                .get("stocks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|stock| alpaca_option::OptionStratStockInput {
                    underlying_symbol: stock
                        .get("underlying_symbol")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string(),
                    quantity: stock.get("quantity").and_then(Value::as_i64).unwrap() as i32,
                    cost_per_share: stock.get("cost_per_share").and_then(Value::as_f64).unwrap(),
                })
                .collect::<Vec<_>>();

            serde_json::to_value(url::build_optionstrat_url(
                &alpaca_option::OptionStratUrlInput {
                    underlying_display_symbol: input
                        .get("underlying_display_symbol")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    legs,
                    stocks,
                },
            ))
            .unwrap()
        }
        "url.parse_optionstrat_url" => serde_json::to_value(
            url::parse_optionstrat_url(input.get("url").unwrap().as_str().unwrap()).unwrap(),
        )
        .unwrap(),
        "url.parse_optionstrat_leg_fragments" => {
            let fragments = input
                .get("leg_fragments")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            serde_json::to_value(
                url::parse_optionstrat_leg_fragments(
                    input
                        .get("underlying_display_symbol")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    &fragments,
                )
                .unwrap(),
            )
            .unwrap()
        }
        "pricing.price_black_scholes" => serde_json::to_value(
            pricing::price_black_scholes(&alpaca_option::BlackScholesInput {
                spot: input.get("spot").unwrap().as_f64().unwrap(),
                strike: input.get("strike").unwrap().as_f64().unwrap(),
                years: input.get("years").unwrap().as_f64().unwrap(),
                rate: input.get("rate").unwrap().as_f64().unwrap(),
                dividend_yield: input.get("dividend_yield").unwrap().as_f64().unwrap(),
                volatility: input.get("volatility").unwrap().as_f64().unwrap(),
                option_right: OptionRight::from_str(
                    input.get("option_right").unwrap().as_str().unwrap(),
                )
                .unwrap(),
            })
            .unwrap(),
        )
        .unwrap(),
        "pricing.greeks_black_scholes" => serde_json::to_value(
            pricing::greeks_black_scholes(&alpaca_option::BlackScholesInput {
                spot: input.get("spot").unwrap().as_f64().unwrap(),
                strike: input.get("strike").unwrap().as_f64().unwrap(),
                years: input.get("years").unwrap().as_f64().unwrap(),
                rate: input.get("rate").unwrap().as_f64().unwrap(),
                dividend_yield: input.get("dividend_yield").unwrap().as_f64().unwrap(),
                volatility: input.get("volatility").unwrap().as_f64().unwrap(),
                option_right: OptionRight::from_str(
                    input.get("option_right").unwrap().as_str().unwrap(),
                )
                .unwrap(),
            })
            .unwrap(),
        )
        .unwrap(),
        "pricing.implied_volatility_from_price" => serde_json::to_value(
            pricing::implied_volatility_from_price(
                &alpaca_option::BlackScholesImpliedVolatilityInput {
                    target_price: input.get("target_price").unwrap().as_f64().unwrap(),
                    spot: input.get("spot").unwrap().as_f64().unwrap(),
                    strike: input.get("strike").unwrap().as_f64().unwrap(),
                    years: input.get("years").unwrap().as_f64().unwrap(),
                    rate: input.get("rate").unwrap().as_f64().unwrap(),
                    dividend_yield: input.get("dividend_yield").unwrap().as_f64().unwrap(),
                    option_right: OptionRight::from_str(
                        input.get("option_right").unwrap().as_str().unwrap(),
                    )
                    .unwrap(),
                    lower_bound: input.get("lower_bound").and_then(Value::as_f64),
                    upper_bound: input.get("upper_bound").and_then(Value::as_f64),
                    tolerance: input.get("tolerance").and_then(Value::as_f64),
                    max_iterations: input
                        .get("max_iterations")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize),
                },
            )
            .unwrap(),
        )
        .unwrap(),
        "pricing.black_scholes_put_call_parity" => black_scholes_put_call_parity(input),
        "math.black76_price" => serde_json::to_value(
            math::black76::price(
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.black76_greeks" => serde_json::to_value(
            math::black76::greeks(
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.black76_implied_volatility_from_price" => serde_json::to_value(
            math::black76::implied_volatility_from_price(
                input.get("target_price").unwrap().as_f64().unwrap(),
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("lower_bound").and_then(Value::as_f64),
                input.get("upper_bound").and_then(Value::as_f64),
                input.get("tolerance").and_then(Value::as_f64),
                input
                    .get("max_iterations")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.bachelier_price" => serde_json::to_value(
            math::bachelier::price(
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("normal_volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.bachelier_greeks" => serde_json::to_value(
            math::bachelier::greeks(
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("normal_volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.bachelier_implied_volatility_from_price" => serde_json::to_value(
            math::bachelier::implied_volatility_from_price(
                input.get("target_price").unwrap().as_f64().unwrap(),
                input.get("forward").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("lower_bound").and_then(Value::as_f64),
                input.get("upper_bound").and_then(Value::as_f64),
                input.get("tolerance").and_then(Value::as_f64),
                input
                    .get("max_iterations")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.american_tree_price" => serde_json::to_value(
            math::american::tree_price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input
                    .get("steps")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
                input.get("use_richardson").and_then(Value::as_bool),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.american_barone_adesi_whaley_price" => serde_json::to_value(
            math::american::barone_adesi_whaley_price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.american_bjerksund_stensland_1993_price" => serde_json::to_value(
            math::american::bjerksund_stensland_1993_price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.american_ju_quadratic_price" => serde_json::to_value(
            math::american::ju_quadratic_price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.american.discrete_dividend_price" => {
            let dividends = input
                .get("dividends")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|dividend| math::american::CashDividend {
                    time: dividend.get("time").unwrap().as_f64().unwrap(),
                    amount: dividend.get("amount").unwrap().as_f64().unwrap(),
                })
                .collect::<Vec<_>>();

            serde_json::to_value(
                math::american::discrete_dividend_price(
                    input.get("spot").unwrap().as_f64().unwrap(),
                    input.get("strike").unwrap().as_f64().unwrap(),
                    input.get("years").unwrap().as_f64().unwrap(),
                    input.get("rate").unwrap().as_f64().unwrap(),
                    input.get("volatility").unwrap().as_f64().unwrap(),
                    input.get("option_right").unwrap().as_str().unwrap(),
                    input.get("cash_dividend_model").unwrap().as_str().unwrap(),
                    &dividends,
                )
                .unwrap(),
            )
            .unwrap()
        }
        "math.barrier.price" => serde_json::to_value(
            math::barrier::price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("barrier").unwrap().as_f64().unwrap(),
                input.get("rebate").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("barrier_type").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "math.geometric_asian.price" => serde_json::to_value(
            math::geometric_asian::price(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("average_style").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "probability.expiry_probability_in_range" => serde_json::to_value(
            probability::expiry_probability_in_range_with_rate(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("lower_price").unwrap().as_f64().unwrap(),
                input.get("upper_price").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.annualized_premium_yield" => serde_json::to_value(
            analysis::annualized_premium_yield(
                input.get("premium").unwrap().as_f64().unwrap(),
                input.get("capital_base").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.calendar_forward_factor" => serde_json::to_value(
            analysis::calendar_forward_factor(
                input.get("short_iv").unwrap().as_f64().unwrap(),
                input.get("long_iv").unwrap().as_f64().unwrap(),
                input.get("short_years").unwrap().as_f64().unwrap(),
                input.get("long_years").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.moneyness_ratio" => serde_json::to_value(
            analysis::moneyness_ratio(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.moneyness_label" => serde_json::to_value(
            analysis::moneyness_label(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("atm_band").and_then(Value::as_f64),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.otm_percent" => serde_json::to_value(
            analysis::otm_percent(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.assignment_risk" => serde_json::to_value(
            analysis::assignment_risk(input.get("extrinsic").unwrap().as_f64().unwrap()).unwrap(),
        )
        .unwrap(),
        "analysis.short_extrinsic_amount" => serde_json::to_value(
            analysis::short_extrinsic_amount(
                input.get("spot").unwrap().as_f64().unwrap(),
                &fixture_positions(input.get("positions").unwrap()),
                input
                    .get("structure_quantity")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.short_itm_positions" => serde_json::to_value(
            analysis::short_itm_positions(
                input.get("spot").unwrap().as_f64().unwrap(),
                &fixture_positions(input.get("positions").unwrap()),
            )
            .unwrap(),
        )
        .unwrap(),
        "analysis.strike_for_target_delta" => serde_json::to_value(
            analysis::strike_for_target_delta(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("years").unwrap().as_f64().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").unwrap().as_f64().unwrap(),
                input.get("volatility").unwrap().as_f64().unwrap(),
                input.get("target_delta").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("strike_step").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "model.option_snapshot" => serde_json::json!({
            "valid": serde_json::from_value::<OptionSnapshot>(input.clone()).is_ok()
        }),
        "model.option_position" => serde_json::json!({
            "valid": serde_json::from_value::<OptionPosition>(input.clone()).is_ok()
        }),
        "model.option_chain_record" => serde_json::json!({
            "valid": serde_json::from_value::<OptionChainRecord>(input.clone()).is_ok()
        }),
        "model.option_chain" => serde_json::json!({
            "valid": serde_json::from_value::<OptionChain>(input.clone()).is_ok()
        }),
        "pricing.intrinsic_value" => serde_json::to_value(
            pricing::intrinsic_value(
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "pricing.extrinsic_value" => serde_json::to_value(
            pricing::extrinsic_value(
                input.get("option_price").unwrap().as_f64().unwrap(),
                input.get("spot").unwrap().as_f64().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("option_right").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.best_worst" => {
            let structure_quantity = input
                .get("structure_quantity")
                .and_then(Value::as_u64)
                .map(|value| value as u32);
            if let Some(positions) = input.get("positions") {
                let positions = fixture_positions(positions);
                serde_json::to_value(
                    execution_quote::best_worst(positions.as_slice(), structure_quantity).unwrap(),
                )
                .unwrap()
            } else {
                let legs =
                    serde_json::from_value::<Vec<QuotedLeg>>(input.get("legs").unwrap().clone())
                        .unwrap();
                serde_json::to_value(
                    execution_quote::best_worst(legs.as_slice(), structure_quantity).unwrap(),
                )
                .unwrap()
            }
        }
        "execution_quote.quote" => {
            if let Some(snapshot) = input.get("snapshot") {
                let snapshot = serde_json::from_value::<OptionSnapshot>(snapshot.clone()).unwrap();
                serde_json::to_value(execution_quote::quote(&snapshot)).unwrap()
            } else if let Some(position) = input.get("position") {
                let position = fixture_position(position);
                serde_json::to_value(execution_quote::quote(&position)).unwrap()
            } else if let Some(leg) = input.get("leg") {
                let leg = serde_json::from_value::<QuotedLeg>(leg.clone()).unwrap();
                serde_json::to_value(execution_quote::quote(&leg)).unwrap()
            } else {
                let quote = serde_json::from_value::<alpaca_option::OptionQuote>(
                    input.get("quote").unwrap().clone(),
                )
                .unwrap();
                serde_json::to_value(execution_quote::quote(&quote)).unwrap()
            }
        }
        "execution_quote.limit_price" => serde_json::to_value(execution_quote::limit_price(
            input.get("price").and_then(Value::as_f64),
        ))
        .unwrap(),
        "execution_quote.order_legs" => serde_json::to_value(
            execution_quote::order_legs(
                &fixture_positions(input.get("positions").unwrap()),
                input.get("action").unwrap().as_str().unwrap(),
                input
                    .get("include_leg_types")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .as_deref(),
                input
                    .get("exclude_leg_types")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .as_deref(),
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.leg" => serde_json::to_value(execution_quote::leg(
            serde_json::from_value::<ExecutionLegInput>(input.clone()).unwrap(),
        ))
        .unwrap(),
        "execution_quote.roll_legs" => serde_json::to_value(
            execution_quote::roll_legs(
                &fixture_positions(input.get("positions").unwrap()),
                &serde_json::from_value::<std::collections::HashMap<String, ExecutionSnapshot>>(
                    input.get("snapshots").unwrap().clone(),
                )
                .unwrap(),
                &serde_json::from_value::<Vec<RollLegSelection>>(
                    input.get("selections").unwrap().clone(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.scale_quote" => serde_json::to_value(
            execution_quote::scale_quote(
                input.get("price").unwrap().as_f64().unwrap(),
                input.get("structure_quantity").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.scale_quote_range" => serde_json::to_value(
            execution_quote::scale_quote_range(
                input.get("best_price").unwrap().as_f64().unwrap(),
                input.get("worst_price").unwrap().as_f64().unwrap(),
                input.get("structure_quantity").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.limit_quote_by_progress" => serde_json::to_value(
            execution_quote::limit_quote_by_progress(
                input.get("best_price").unwrap().as_f64().unwrap(),
                input.get("worst_price").unwrap().as_f64().unwrap(),
                input.get("progress").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "execution_quote.progress_of_limit" => serde_json::to_value(
            execution_quote::progress_of_limit(
                input.get("best_price").unwrap().as_f64().unwrap(),
                input.get("worst_price").unwrap().as_f64().unwrap(),
                input.get("limit_price").unwrap().as_f64().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.single_leg_payoff_at_expiry" => serde_json::to_value(
            payoff::single_leg_payoff_at_expiry(
                input.get("option_right").unwrap().as_str().unwrap(),
                input.get("position_side").unwrap().as_str().unwrap(),
                input.get("strike").unwrap().as_f64().unwrap(),
                input.get("premium").unwrap().as_f64().unwrap(),
                input.get("quantity").unwrap().as_u64().unwrap() as u32,
                input
                    .get("underlying_price_at_expiry")
                    .unwrap()
                    .as_f64()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.strategy_payoff_at_expiry" => serde_json::to_value(
            payoff::strategy_payoff_at_expiry(
                &serde_json::from_value::<Vec<PayoffLegInput>>(input.get("legs").unwrap().clone())
                    .unwrap(),
                input
                    .get("underlying_price_at_expiry")
                    .unwrap()
                    .as_f64()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.break_even_points" => serde_json::to_value(
            payoff::break_even_points(
                &serde_json::from_value::<Vec<PayoffLegInput>>(input.get("legs").unwrap().clone())
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.strategy_pnl" => serde_json::to_value(
            payoff::strategy_pnl(
                &serde_json::from_value::<StrategyPnlInput>(input.clone()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.strategy_break_even_points" => serde_json::to_value(
            payoff::strategy_break_even_points(
                &serde_json::from_value::<StrategyBreakEvenInput>(input.clone()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.option_strategy_expiration_time" => serde_json::to_value(
            OptionStrategy::expiration_time(
                &serde_json::from_value::<Vec<StrategyValuationPosition>>(
                    input.get("positions").unwrap().clone(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.option_strategy_snapshot_greeks" => serde_json::to_value(
            OptionStrategy::aggregate_snapshot_greeks(
                &fixture_positions(input.get("positions").unwrap()),
                input
                    .get("strategy_quantity")
                    .unwrap()
                    .as_f64()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.option_strategy_model_greeks" => serde_json::to_value(
            OptionStrategy::aggregate_model_greeks(
                &serde_json::from_value::<Vec<StrategyValuationPosition>>(
                    input.get("positions").unwrap().clone(),
                )
                .unwrap(),
                input.get("underlying_price").unwrap().as_f64().unwrap(),
                input.get("evaluation_time").unwrap().as_str().unwrap(),
                input.get("rate").unwrap().as_f64().unwrap(),
                input.get("dividend_yield").and_then(Value::as_f64),
                input
                    .get("long_volatility_shift")
                    .and_then(Value::as_f64),
                input
                    .get("strategy_quantity")
                    .unwrap()
                    .as_f64()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "payoff.option_strategy_curve" => {
            let strategy =
                OptionStrategy::from_input(&serde_json::from_value::<OptionStrategyInput>(
                    input.clone(),
                )
                .unwrap())
                .unwrap();
            serde_json::to_value(
                strategy
                    .sample_curve(
                        input.get("lower_bound").unwrap().as_f64().unwrap(),
                        input.get("upper_bound").unwrap().as_f64().unwrap(),
                        input.get("step").unwrap().as_f64().unwrap(),
                    )
                    .unwrap(),
            )
            .unwrap()
        }
        "expiration_selection.nearest_weekly_expiration" => serde_json::to_value(
            expiration_selection::nearest_weekly_expiration(
                input.get("anchor_date").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "expiration_selection.weekly_expirations_between" => serde_json::to_value(
            expiration_selection::weekly_expirations_between(
                input.get("start_date").unwrap().as_str().unwrap(),
                input.get("end_date").unwrap().as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "expiration_selection.standard_monthly_expiration" => serde_json::to_value(
            expiration_selection::standard_monthly_expiration(
                input.get("year").unwrap().as_i64().unwrap() as i32,
                input.get("month").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "numeric.normal_cdf" => serde_json::to_value(numeric::normal_cdf(
            input.get("x").unwrap().as_f64().unwrap(),
        ))
        .unwrap(),
        "numeric.normal_pdf" => serde_json::to_value(numeric::normal_pdf(
            input.get("x").unwrap().as_f64().unwrap(),
        ))
        .unwrap(),
        "numeric.round" => serde_json::to_value(
            numeric::round(
                input.get("value").unwrap().as_f64().unwrap(),
                input.get("decimals").unwrap().as_u64().unwrap() as u32,
            )
            .unwrap(),
        )
        .unwrap(),
        "numeric.linspace" => serde_json::to_value(
            numeric::linspace(
                input.get("start").unwrap().as_f64().unwrap(),
                input.get("end").unwrap().as_f64().unwrap(),
                input.get("count").unwrap().as_u64().unwrap() as usize,
            )
            .unwrap(),
        )
        .unwrap(),
        "numeric.brent_solve" => {
            let evaluator = input.get("evaluator").unwrap().as_str().unwrap();
            let solve = |value: f64| match evaluator {
                "square_minus_two" => value * value - 2.0,
                other => panic!("Unhandled numeric evaluator: {other}"),
            };
            serde_json::to_value(
                numeric::brent_solve(
                    input.get("lower_bound").unwrap().as_f64().unwrap(),
                    input.get("upper_bound").unwrap().as_f64().unwrap(),
                    solve,
                    input.get("tolerance").and_then(Value::as_f64),
                    input
                        .get("max_iterations")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize),
                )
                .unwrap(),
            )
            .unwrap()
        }
        other => panic!("Unhandled fixture api: {other}"),
    }
}

#[test]
fn rate_dependent_support_fixtures_use_explicit_rates() {
    for fixture_path in load_support_fixture_paths() {
        assert_rate_dependent_cases_have_explicit_rate(&fixture_path);
    }
}

#[test]
fn rate_dependent_layer_fixtures_use_explicit_rates() {
    for fixture_path in load_layer_fixture_paths() {
        assert_rate_dependent_cases_have_explicit_rate(&fixture_path);
    }
}

#[test]
fn fixture_suite() {
    for fixture_path in load_support_fixture_paths() {
        for case in load_cases(&fixture_path) {
            let actual = run_case(&case);
            let expected = unwrap_expected(case.get("expected").unwrap());
            let tolerance = case.get("tolerance").and_then(Value::as_f64);
            let case_label = format!(
                "{}::{}",
                fixture_path,
                case.get("id").and_then(Value::as_str).unwrap()
            );
            let field_tolerances = case
                .get("field_tolerances")
                .cloned()
                .map(serde_json::from_value::<HashMap<String, f64>>)
                .transpose()
                .expect("field_tolerances should be an object of numbers");
            assert_with_tolerance(
                &actual,
                &expected,
                tolerance,
                &case_label,
                field_tolerances.as_ref(),
                &[],
            );
        }
    }
}

#[test]
#[ignore = "slow layer-wide numerical regression suite; run explicitly when auditing fixture datasets"]
fn layer_fixture_suite() {
    for fixture_path in load_layer_fixture_paths() {
        for case in load_cases(&fixture_path) {
            let actual = run_case(&case);
            let expected = unwrap_expected(case.get("expected").unwrap());
            let tolerance = case.get("tolerance").and_then(Value::as_f64);
            let case_label = format!(
                "{}::{}",
                fixture_path,
                case.get("id").and_then(Value::as_str).unwrap()
            );
            let field_tolerances = case
                .get("field_tolerances")
                .cloned()
                .map(serde_json::from_value::<HashMap<String, f64>>)
                .transpose()
                .expect("field_tolerances should be an object of numbers");
            assert_with_tolerance(
                &actual,
                &expected,
                tolerance,
                &case_label,
                field_tolerances.as_ref(),
                &[],
            );
        }
    }
}
