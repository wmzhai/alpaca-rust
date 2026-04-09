#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_data::{
    Client as DataClient,
    options::{ChainRequest, OptionsFeed, Snapshot, SnapshotsRequest},
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
    pub(crate) deep_resting_limit_price: Decimal,
    pub(crate) marketable_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SingleLegOptionOrderContext {
    pub(crate) underlying_symbol: String,
    pub(crate) contract_symbol: String,
    pub(crate) non_marketable_limit_price: Decimal,
    pub(crate) more_conservative_limit_price: Decimal,
    pub(crate) marketable_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockOrderPriceContext {
    pub(crate) bid: Decimal,
    pub(crate) ask: Decimal,
    pub(crate) resting_buy_limit_price: Decimal,
    pub(crate) non_marketable_buy_limit_price: Decimal,
    pub(crate) resting_sell_limit_price: Decimal,
    pub(crate) resting_buy_stop_price: Decimal,
    pub(crate) resting_buy_stop_limit_price: Decimal,
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

pub(crate) fn unique_client_order_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    )
}

pub(crate) async fn clear_option_universe_cache() {
    option_universe_cache().lock().await.clear();
}

pub(crate) async fn non_marketable_buy_limit_price(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<Decimal, String> {
    Ok(stock_order_price_context(data_client, underlying_symbol)
        .await?
        .non_marketable_buy_limit_price)
}

pub(crate) async fn stock_order_price_context(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<StockOrderPriceContext, String> {
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

    let bid = quote.bp.or(quote.ap).ok_or_else(|| {
        format!("stock snapshot for {underlying_symbol} did not include bid or ask price")
    })?;
    let ask = quote.ap.or(quote.bp).ok_or_else(|| {
        format!("stock snapshot for {underlying_symbol} did not include ask or bid price")
    })?;
    let resting_buy_stop_price = conservative_price_above_market(ask.max(Decimal::new(1, 2)));
    let resting_buy_stop_limit_price =
        conservative_price_above_market(resting_buy_stop_price.max(Decimal::new(1, 2)));

    Ok(StockOrderPriceContext {
        bid,
        ask,
        resting_buy_limit_price: resting_buy_limit_price(bid, ask),
        non_marketable_buy_limit_price: conservative_price_below_market(
            bid.max(Decimal::new(1, 2)),
        ),
        resting_sell_limit_price: conservative_price_above_market(ask.max(Decimal::new(1, 2))),
        resting_buy_stop_price,
        resting_buy_stop_limit_price,
    })
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

pub(crate) async fn discover_single_leg_call(
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<SingleLegOptionOrderContext, String> {
    let universe = discover_option_universe(data_client, underlying_symbol).await?;
    let mut calls = contracts_for_type(&universe.quoted_contracts, OptionContractType::Call);
    calls.sort_by(|left, right| {
        single_leg_sort_key(left, universe.spot).cmp(&single_leg_sort_key(right, universe.spot))
    });

    for contract in calls {
        if let Ok(context) = build_single_leg_context(underlying_symbol, contract) {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted single-leg call for {underlying_symbol} with a distinct replace price"
    ))
}

pub(crate) async fn current_mleg_replacement_limit_price(
    data_client: &DataClient,
    legs: &[OptionLegRequest],
    request_side: OrderSide,
) -> Result<Decimal, String> {
    let symbols = legs
        .iter()
        .map(|leg| leg.symbol.clone())
        .collect::<Vec<_>>();
    let snapshots = data_client
        .options()
        .snapshots(SnapshotsRequest {
            symbols,
            feed: Some(OptionsFeed::Indicative),
            limit: Some(legs.len() as u32),
            page_token: None,
        })
        .await
        .map_err(|error| format!("option snapshots request failed: {error}"))?;
    let current_mid = current_mleg_mid_price(legs, &request_side, &snapshots.snapshots)?;
    let non_marketable_limit_price =
        conservative_price_below_market(current_mid.max(Decimal::new(1, 2)));
    let more_conservative_limit_price = distinct_more_conservative_limit_price(
        non_marketable_limit_price,
        "current multi-leg replacement",
    )?;

    distinct_more_conservative_limit_price(
        more_conservative_limit_price,
        "current multi-leg replacement",
    )
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

fn current_mleg_mid_price(
    legs: &[OptionLegRequest],
    request_side: &OrderSide,
    snapshots: &HashMap<String, Snapshot>,
) -> Result<Decimal, String> {
    let raw_total = legs.iter().try_fold(Decimal::ZERO, |total, leg| {
        let side = leg
            .side
            .clone()
            .ok_or_else(|| format!("multi-leg request for {} did not include side", leg.symbol))?;
        let leg_mid_price = option_mid_price_from_snapshot(
            leg.symbol.as_str(),
            snapshots.get(&leg.symbol).ok_or_else(|| {
                format!(
                    "option snapshots response did not include multi-leg leg {}",
                    leg.symbol
                )
            })?,
        )?;
        let ratio_qty = Decimal::from(leg.ratio_qty);
        let contribution = match side {
            OrderSide::Buy => leg_mid_price * ratio_qty,
            OrderSide::Sell => -(leg_mid_price * ratio_qty),
            OrderSide::Unspecified => {
                return Err(format!(
                    "multi-leg request for {} used an unspecified side",
                    leg.symbol
                ));
            }
        };

        Ok(total + contribution)
    })?;

    let normalized_total = match request_side {
        OrderSide::Buy | OrderSide::Unspecified => raw_total,
        OrderSide::Sell => -raw_total,
    }
    .round_dp(2);
    if normalized_total <= MIN_PRICE {
        return Err(format!(
            "current multi-leg replacement debit {normalized_total} was not positive"
        ));
    }

    Ok(normalized_total)
}

fn option_mid_price_from_snapshot(symbol: &str, snapshot: &Snapshot) -> Result<Decimal, String> {
    let latest_trade_price = snapshot.latest_trade.as_ref().and_then(|trade| trade.p);
    let bid = snapshot
        .latest_quote
        .as_ref()
        .and_then(|quote| quote.bp)
        .or(latest_trade_price)
        .ok_or_else(|| format!("option snapshot for {symbol} did not include bid or trade"))?;
    let ask = snapshot
        .latest_quote
        .as_ref()
        .and_then(|quote| quote.ap.or(quote.bp))
        .or(latest_trade_price)
        .ok_or_else(|| format!("option snapshot for {symbol} did not include ask or trade"))?;

    if bid <= MIN_PRICE || ask <= MIN_PRICE || ask < bid {
        return Err(format!(
            "option snapshot for {symbol} did not expose a usable bid/ask pair"
        ));
    }

    Ok(((bid + ask) / Decimal::new(2, 0)).round_dp(2))
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
    _spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            if let Ok(context) = build_debit_mleg_context(
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
            ) {
                return Ok(context);
            }
        }
    }

    Err(format!(
        "failed to discover a quoted debit call spread for {underlying_symbol}"
    ))
}

fn find_put_spread(
    underlying_symbol: &str,
    _spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            if let Ok(context) = build_debit_mleg_context(
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
            ) {
                return Ok(context);
            }
        }
    }

    Err(format!(
        "failed to discover a quoted debit put spread for {underlying_symbol}"
    ))
}

fn find_iron_condor(
    underlying_symbol: &str,
    _spot: Decimal,
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

        let put_candidates = expiration_puts;
        let call_candidates = expiration_calls;

        if put_candidates.len() < 2 || call_candidates.len() < 2 {
            continue;
        }

        for outer_put_index in 0..put_candidates.len() - 1 {
            for inner_put_index in outer_put_index + 1..put_candidates.len() {
                let outer_put = put_candidates[outer_put_index].clone();
                let inner_put = put_candidates[inner_put_index].clone();

                for inner_call_index in 0..call_candidates.len() - 1 {
                    for outer_call_index in inner_call_index + 1..call_candidates.len() {
                        let inner_call = call_candidates[inner_call_index].clone();
                        let outer_call = call_candidates[outer_call_index].clone();
                        if let Ok(context) = build_debit_mleg_context(
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
                        ) {
                            return Ok(context);
                        }
                    }
                }
            }
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

fn single_leg_sort_key(
    contract: &QuotedOptionContract,
    spot: Decimal,
) -> (String, Decimal, String) {
    (
        contract.contract.expiration_date.clone(),
        (contract.contract.strike_price - spot).abs(),
        contract.contract.symbol.clone(),
    )
}

fn build_single_leg_context(
    underlying_symbol: &str,
    contract: QuotedOptionContract,
) -> Result<SingleLegOptionOrderContext, String> {
    let non_marketable_limit_price =
        conservative_price_below_market(contract.bid.max(Decimal::new(1, 2)));
    let more_conservative_limit_price = distinct_more_conservative_limit_price(
        non_marketable_limit_price,
        &format!("single-leg option contract {}", contract.contract.symbol),
    )?;
    let marketable_limit_price = contract.ask.max(Decimal::new(1, 2)).round_dp(2);

    if marketable_limit_price <= MIN_PRICE {
        return Err(format!(
            "single-leg option contract {} did not have a usable marketable limit price",
            contract.contract.symbol
        ));
    }

    Ok(SingleLegOptionOrderContext {
        underlying_symbol: underlying_symbol.to_owned(),
        contract_symbol: contract.contract.symbol,
        non_marketable_limit_price,
        more_conservative_limit_price,
        marketable_limit_price,
    })
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
    let more_conservative_limit_price = distinct_more_conservative_limit_price(
        non_marketable_limit_price,
        &format!("multi-leg strategy for {underlying_symbol}"),
    )?;
    let deep_resting_limit_price = distinct_more_conservative_limit_price(
        more_conservative_limit_price,
        &format!("multi-leg strategy for {underlying_symbol}"),
    )?;
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
        deep_resting_limit_price,
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

fn conservative_price_above_market(price: Decimal) -> Decimal {
    (price * Decimal::new(105, 2)).round_dp(2) + Decimal::new(1, 2)
}

fn resting_buy_limit_price(bid: Decimal, ask: Decimal) -> Decimal {
    let minimum_tick = Decimal::new(1, 2);
    let near_ask = (ask - minimum_tick).round_dp(2);
    if near_ask > MIN_PRICE && near_ask < ask {
        return near_ask;
    }
    if bid > MIN_PRICE && bid < ask {
        return bid.round_dp(2);
    }

    conservative_price_below_market(ask.max(minimum_tick))
}

fn distinct_more_conservative_limit_price(
    non_marketable_limit_price: Decimal,
    subject: &str,
) -> Result<Decimal, String> {
    let candidate = conservative_price_below_market(non_marketable_limit_price);
    if candidate < non_marketable_limit_price {
        Ok(candidate)
    } else {
        Err(format!(
            "{subject} did not produce a distinct replace price"
        ))
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
}
