#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

use alpaca_data::{
    Client as DataClient,
    options::{ChainRequest, OptionsFeed, Snapshot},
    stocks::{DataFeed, SnapshotRequest},
};
use alpaca_trade::orders::{OptionLegRequest, OrderSide, PositionIntent};
use rust_decimal::Decimal;
use tokio::sync::Mutex;

const MIN_PRICE: Decimal = Decimal::ZERO;
static OPTION_UNIVERSE_CACHE: OnceLock<Mutex<HashMap<String, CachedOptionUniverse>>> =
    OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionContractType {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedOptionContract {
    symbol: String,
    expiration_date: String,
    contract_type: OptionContractType,
    strike_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MultiLegOrderContext {
    pub(crate) underlying_symbol: String,
    pub(crate) legs: Vec<OptionLegRequest>,
    pub(crate) non_marketable_limit_price: Decimal,
    pub(crate) more_conservative_limit_price: Decimal,
    pub(crate) marketable_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
struct QuotedOptionContract {
    contract: ObservedOptionContract,
    bid: Decimal,
    ask: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
struct CachedOptionUniverse {
    spot: Decimal,
    quoted_contracts: Vec<QuotedOptionContract>,
}

#[derive(Debug, Clone, PartialEq)]
struct StrategyLeg {
    contract: QuotedOptionContract,
    ratio_qty: u32,
    side: OrderSide,
    position_intent: PositionIntent,
}

pub(crate) async fn discover_mleg_call_spread(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let universe = discover_option_universe(data_client, underlying_symbol).await?;
    let calls = contracts_for_type(&universe.quoted_contracts, OptionContractType::Call);

    find_call_spread(underlying_symbol, universe.spot, calls)
}

pub(crate) async fn discover_mleg_put_spread(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let universe = discover_option_universe(data_client, underlying_symbol).await?;
    let puts = contracts_for_type(&universe.quoted_contracts, OptionContractType::Put);

    find_put_spread(underlying_symbol, universe.spot, puts)
}

pub(crate) async fn discover_mleg_iron_condor(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let universe = discover_option_universe(data_client, underlying_symbol).await?;
    let puts = contracts_for_type(&universe.quoted_contracts, OptionContractType::Put);
    let calls = contracts_for_type(&universe.quoted_contracts, OptionContractType::Call);

    find_iron_condor(underlying_symbol, universe.spot, puts, calls)
}

fn option_universe_cache() -> &'static Mutex<HashMap<String, CachedOptionUniverse>> {
    OPTION_UNIVERSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn discover_option_universe(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<CachedOptionUniverse, String> {
    {
        let cache = option_universe_cache().lock().await;
        if let Some(cached) = cache.get(underlying_symbol).cloned() {
            return Ok(cached);
        }
    }

    let loaded = fetch_option_universe(data_client, underlying_symbol).await?;
    let mut cache = option_universe_cache().lock().await;
    let cached = cache
        .entry(underlying_symbol.to_owned())
        .or_insert_with(|| loaded.clone())
        .clone();

    Ok(cached)
}

async fn fetch_option_universe(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<CachedOptionUniverse, String> {
    let spot = latest_stock_ask(data_client, underlying_symbol).await?;
    let response = data_client
        .options()
        .chain_all(ChainRequest {
            underlying_symbol: underlying_symbol.to_owned(),
            feed: Some(OptionsFeed::Indicative),
            limit: Some(1_000),
            ..ChainRequest::default()
        })
        .await
        .map_err(|error| format!("option chain request failed: {error}"))?;

    let mut quoted_contracts = response
        .snapshots
        .into_iter()
        .filter_map(|(symbol, snapshot)| {
            quoted_contract_from_snapshot(underlying_symbol, symbol.as_str(), snapshot)
        })
        .collect::<Vec<_>>();
    quoted_contracts.sort_by(|left, right| left.contract.symbol.cmp(&right.contract.symbol));

    if quoted_contracts.is_empty() {
        return Err(format!(
            "option chain returned no quoted contracts for {underlying_symbol}"
        ));
    }

    Ok(CachedOptionUniverse {
        spot,
        quoted_contracts,
    })
}

async fn latest_stock_ask(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<Decimal, String> {
    let snapshot = data_client
        .stocks()
        .snapshot(SnapshotRequest {
            symbol: underlying_symbol.to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .map_err(|error| format!("stock snapshot request failed: {error}"))?;
    let quote = snapshot.latest_quote.ok_or_else(|| {
        format!("stock snapshot for {underlying_symbol} did not include latest_quote")
    })?;

    quote.ap.or(quote.bp).ok_or_else(|| {
        format!("stock snapshot for {underlying_symbol} did not include ask or bid price")
    })
}

fn quoted_contract_from_snapshot(
    underlying_symbol: &str,
    symbol: &str,
    snapshot: Snapshot,
) -> Option<QuotedOptionContract> {
    let contract = parse_occ_option_symbol(symbol).ok()?;
    if !symbol.starts_with(underlying_symbol) {
        return None;
    }

    let latest_trade_price = snapshot.latest_trade.as_ref().and_then(|trade| trade.p);
    let bid = snapshot
        .latest_quote
        .as_ref()
        .and_then(|quote| quote.bp)
        .or(latest_trade_price)?;
    let ask = snapshot
        .latest_quote
        .as_ref()
        .and_then(|quote| quote.ap.or(quote.bp))
        .or(latest_trade_price)?;

    if bid <= MIN_PRICE || ask <= MIN_PRICE || ask < bid {
        return None;
    }

    Some(QuotedOptionContract { contract, bid, ask })
}

fn parse_occ_option_symbol(symbol: &str) -> Result<ObservedOptionContract, String> {
    if symbol.len() <= 15 {
        return Err(format!(
            "option symbol {symbol} is shorter than the OCC contract suffix"
        ));
    }

    let root_end = symbol.len() - 15;
    let suffix = &symbol[root_end..];
    let expiration_date = format!("20{}-{}-{}", &suffix[0..2], &suffix[2..4], &suffix[4..6]);
    let contract_type = match &suffix[6..7] {
        "C" => OptionContractType::Call,
        "P" => OptionContractType::Put,
        value => {
            return Err(format!(
                "option symbol {symbol} contained an unknown contract type marker {value}"
            ));
        }
    };
    let strike_suffix = suffix[7..15].parse::<i64>().map_err(|error| {
        format!("option symbol {symbol} contained an invalid strike suffix: {error}")
    })?;

    Ok(ObservedOptionContract {
        symbol: symbol.to_owned(),
        expiration_date,
        contract_type,
        strike_price: Decimal::new(strike_suffix, 3),
    })
}

fn contracts_for_type(
    contracts: &[QuotedOptionContract],
    contract_type: OptionContractType,
) -> Vec<QuotedOptionContract> {
    contracts
        .iter()
        .filter(|contract| contract.contract.contract_type == contract_type)
        .cloned()
        .collect()
}

fn find_call_spread(
    underlying_symbol: &str,
    spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        let mut best_candidate = None;
        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            let score = (lower.contract.strike_price - spot).abs();
            let candidate = build_debit_mleg_context(
                underlying_symbol,
                vec![
                    strategy_leg(lower.clone(), 1, OrderSide::Buy, PositionIntent::BuyToOpen),
                    strategy_leg(
                        higher.clone(),
                        1,
                        OrderSide::Sell,
                        PositionIntent::SellToOpen,
                    ),
                ],
            );
            if let Ok(context) = candidate {
                match &best_candidate {
                    Some((best_score, _)) if score >= *best_score => {}
                    _ => best_candidate = Some((score, context)),
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit call spread for {underlying_symbol}"
    ))
}

fn find_put_spread(
    underlying_symbol: &str,
    spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        let mut best_candidate = None;
        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            let score = (higher.contract.strike_price - spot).abs();
            let candidate = build_debit_mleg_context(
                underlying_symbol,
                vec![
                    strategy_leg(higher.clone(), 1, OrderSide::Buy, PositionIntent::BuyToOpen),
                    strategy_leg(
                        lower.clone(),
                        1,
                        OrderSide::Sell,
                        PositionIntent::SellToOpen,
                    ),
                ],
            );
            if let Ok(context) = candidate {
                match &best_candidate {
                    Some((best_score, _)) if score >= *best_score => {}
                    _ => best_candidate = Some((score, context)),
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit put spread for {underlying_symbol}"
    ))
}

fn find_iron_condor(
    underlying_symbol: &str,
    spot: Decimal,
    puts: Vec<QuotedOptionContract>,
    calls: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    let put_groups = group_by_expiration(puts);
    let call_groups = group_by_expiration(calls)
        .into_iter()
        .collect::<HashMap<_, _>>();

    for (expiration, mut expiration_puts) in put_groups {
        let Some(mut expiration_calls) = call_groups.get(&expiration).cloned() else {
            continue;
        };

        sort_by_strike(&mut expiration_puts);
        sort_by_strike(&mut expiration_calls);

        let put_candidates = expiration_puts
            .iter()
            .filter(|contract| contract.contract.strike_price < spot)
            .cloned()
            .collect::<Vec<_>>();
        let call_candidates = expiration_calls
            .iter()
            .filter(|contract| contract.contract.strike_price > spot)
            .cloned()
            .collect::<Vec<_>>();

        if put_candidates.len() < 2 || call_candidates.len() < 2 {
            continue;
        }

        let mut best_candidate = None;
        for outer_put_index in 0..put_candidates.len() - 1 {
            for inner_put_index in outer_put_index + 1..put_candidates.len() {
                let outer_put = put_candidates[outer_put_index].clone();
                let inner_put = put_candidates[inner_put_index].clone();

                for inner_call_index in 0..call_candidates.len() - 1 {
                    for outer_call_index in inner_call_index + 1..call_candidates.len() {
                        let inner_call = call_candidates[inner_call_index].clone();
                        let outer_call = call_candidates[outer_call_index].clone();
                        let score = (spot - inner_put.contract.strike_price).abs()
                            + (inner_call.contract.strike_price - spot).abs()
                            + (inner_put.contract.strike_price - outer_put.contract.strike_price)
                                .abs()
                            + (outer_call.contract.strike_price - inner_call.contract.strike_price)
                                .abs();

                        let candidate = build_debit_mleg_context(
                            underlying_symbol,
                            vec![
                                strategy_leg(
                                    outer_put.clone(),
                                    1,
                                    OrderSide::Sell,
                                    PositionIntent::SellToOpen,
                                ),
                                strategy_leg(
                                    inner_put.clone(),
                                    1,
                                    OrderSide::Buy,
                                    PositionIntent::BuyToOpen,
                                ),
                                strategy_leg(
                                    inner_call.clone(),
                                    1,
                                    OrderSide::Buy,
                                    PositionIntent::BuyToOpen,
                                ),
                                strategy_leg(
                                    outer_call.clone(),
                                    1,
                                    OrderSide::Sell,
                                    PositionIntent::SellToOpen,
                                ),
                            ],
                        );

                        if let Ok(context) = candidate {
                            match &best_candidate {
                                Some((best_score, _)) if score >= *best_score => {}
                                _ => best_candidate = Some((score, context)),
                            }
                        }
                    }
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit iron condor for {underlying_symbol}"
    ))
}

fn strategy_leg(
    contract: QuotedOptionContract,
    ratio_qty: u32,
    side: OrderSide,
    position_intent: PositionIntent,
) -> StrategyLeg {
    StrategyLeg {
        contract,
        ratio_qty,
        side,
        position_intent,
    }
}

fn build_debit_mleg_context(
    underlying_symbol: &str,
    legs: Vec<StrategyLeg>,
) -> Result<MultiLegOrderContext, String> {
    let best_debit = legs
        .iter()
        .map(best_case_debit_contribution)
        .sum::<Decimal>()
        .round_dp(2);
    let worst_debit = legs
        .iter()
        .map(worst_case_debit_contribution)
        .sum::<Decimal>()
        .round_dp(2);

    if worst_debit <= MIN_PRICE {
        return Err(format!(
            "discovered multi-leg strategy for {underlying_symbol} was not a net debit"
        ));
    }

    let non_marketable_limit_price =
        conservative_price_below_market(best_debit.max(Decimal::new(1, 2)));
    let more_conservative_limit_price = conservative_price_below_market(non_marketable_limit_price);
    let marketable_limit_price = (worst_debit + Decimal::new(10, 2)).round_dp(2);

    Ok(MultiLegOrderContext {
        underlying_symbol: underlying_symbol.to_owned(),
        legs: legs
            .into_iter()
            .map(|leg| OptionLegRequest {
                symbol: leg.contract.contract.symbol,
                ratio_qty: leg.ratio_qty,
                side: Some(leg.side),
                position_intent: Some(leg.position_intent),
            })
            .collect(),
        non_marketable_limit_price,
        more_conservative_limit_price,
        marketable_limit_price,
    })
}

fn best_case_debit_contribution(leg: &StrategyLeg) -> Decimal {
    let quantity = Decimal::from(leg.ratio_qty);
    match leg.side {
        OrderSide::Buy => leg.contract.bid * quantity,
        OrderSide::Sell => -(leg.contract.ask * quantity),
        OrderSide::Unspecified => Decimal::ZERO,
    }
}

fn worst_case_debit_contribution(leg: &StrategyLeg) -> Decimal {
    let quantity = Decimal::from(leg.ratio_qty);
    match leg.side {
        OrderSide::Buy => leg.contract.ask * quantity,
        OrderSide::Sell => -(leg.contract.bid * quantity),
        OrderSide::Unspecified => Decimal::ZERO,
    }
}

fn conservative_price_below_market(price: Decimal) -> Decimal {
    let floor = Decimal::new(1, 2);
    let scaled = price * Decimal::new(5, 1);
    if scaled < floor {
        floor
    } else {
        scaled.round_dp(2)
    }
}

fn group_by_expiration(
    contracts: Vec<QuotedOptionContract>,
) -> Vec<(String, Vec<QuotedOptionContract>)> {
    let mut grouped = HashMap::<String, Vec<QuotedOptionContract>>::new();
    for contract in contracts {
        grouped
            .entry(contract.contract.expiration_date.clone())
            .or_default()
            .push(contract);
    }

    let mut grouped = grouped.into_iter().collect::<Vec<_>>();
    grouped.sort_by(|left, right| left.0.cmp(&right.0));
    grouped
}

fn sort_by_strike(contracts: &mut [QuotedOptionContract]) {
    contracts.sort_by(|left, right| left.contract.strike_price.cmp(&right.contract.strike_price));
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
