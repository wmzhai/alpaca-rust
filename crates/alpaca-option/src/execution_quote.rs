use std::collections::{HashMap, HashSet};

use alpaca_core::float;
use alpaca_time::clock;

use crate::contract;
use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::types::{
    ExecutionAction, ExecutionLeg, ExecutionQuoteRange, OptionPosition, OptionQuote,
    OptionSnapshot, OrderSide, PositionIntent, PositionSide, QuotedLeg, RollRequest,
    ScaledExecutionQuote, ScaledExecutionQuoteRange,
};

pub use crate::types::{
    ExecutionLegInput, ExecutionSnapshot, Greeks, GreeksInput, RollLegSelection,
};

const CONTRACT_MULTIPLIER: f64 = 100.0;

pub trait QuoteLike {
    fn quote(&self) -> OptionQuote;
}

impl<T: QuoteLike + ?Sized> QuoteLike for &T {
    fn quote(&self) -> OptionQuote {
        (*self).quote()
    }
}

impl QuoteLike for OptionQuote {
    fn quote(&self) -> OptionQuote {
        self.clone()
    }
}

impl QuoteLike for OptionSnapshot {
    fn quote(&self) -> OptionQuote {
        self.quote.clone()
    }
}

impl QuoteLike for OptionPosition {
    fn quote(&self) -> OptionQuote {
        self.snapshot_ref()
            .map(|snapshot| snapshot.quote.clone())
            .unwrap_or(OptionQuote {
                bid: None,
                ask: None,
                mark: None,
                last: None,
            })
    }
}

impl QuoteLike for QuotedLeg {
    fn quote(&self) -> OptionQuote {
        self.quote.clone()
    }
}

pub trait QuoteRangeLike {
    fn quote_range(&self) -> OptionResult<ExecutionQuoteRange>;
}

impl<T: QuoteRangeLike + ?Sized> QuoteRangeLike for &T {
    fn quote_range(&self) -> OptionResult<ExecutionQuoteRange> {
        (*self).quote_range()
    }
}

impl QuoteRangeLike for [OptionPosition] {
    fn quote_range(&self) -> OptionResult<ExecutionQuoteRange> {
        range_from_positions(self)
    }
}

impl QuoteRangeLike for [QuotedLeg] {
    fn quote_range(&self) -> OptionResult<ExecutionQuoteRange> {
        range_from_legs(self)
    }
}

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_execution_quote_input",
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn round_price(value: f64) -> OptionResult<f64> {
    ensure_finite("quote value", value)?;
    Ok(float::round(value, 2))
}

fn quote_value(value: Option<f64>, name: &str) -> OptionResult<f64> {
    match value {
        Some(number) => {
            ensure_finite(name, number)?;
            Ok(number)
        }
        None => Ok(0.0),
    }
}

fn normalized_mark(quote: &OptionQuote) -> Option<f64> {
    if let Some(mark) = quote.mark {
        return Some(mark);
    }

    match (quote.bid, quote.ask) {
        (Some(bid), Some(ask)) => Some(float::round((bid + ask) / 2.0, 12)),
        (Some(bid), None) => Some(bid),
        (None, Some(ask)) => Some(ask),
        (None, None) => quote.last,
    }
}

pub fn quote(source: &(impl QuoteLike + ?Sized)) -> OptionQuote {
    let base = source.quote();
    let mark = normalized_mark(&base);
    OptionQuote {
        bid: base.bid,
        ask: base.ask,
        mark,
        last: base.last.or(mark),
    }
}

pub fn limit_price(limit_price: Option<f64>) -> f64 {
    match limit_price {
        Some(value) if value.is_finite() => value,
        _ => 0.0,
    }
}

fn quote_bid_ask(option_quote: &OptionQuote) -> OptionResult<(f64, f64)> {
    let normalized = quote(option_quote);
    Ok((
        quote_value(normalized.bid, "bid")?,
        quote_value(normalized.ask, "ask")?,
    ))
}

fn clamp_progress(progress: f64) -> OptionResult<f64> {
    ensure_finite("progress", progress)?;
    let normalized = if progress.abs() > 1.0 {
        progress / 100.0
    } else {
        progress
    };
    Ok(normalized.clamp(0.0, 1.0))
}

fn normalized_filter_set(values: Option<&[String]>) -> HashSet<String> {
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn derived_leg_type(position: &OptionPosition) -> String {
    position.leg_type()
}

fn order_side_for_action(position_side: &PositionSide, action: &ExecutionAction) -> OrderSide {
    match (position_side, action) {
        (PositionSide::Long, ExecutionAction::Open) => OrderSide::Buy,
        (PositionSide::Long, ExecutionAction::Close) => OrderSide::Sell,
        (PositionSide::Short, ExecutionAction::Open) => OrderSide::Sell,
        (PositionSide::Short, ExecutionAction::Close) => OrderSide::Buy,
    }
}

fn position_intent_for(side: &OrderSide, action: &ExecutionAction) -> PositionIntent {
    match (side, action) {
        (OrderSide::Buy, ExecutionAction::Open) => PositionIntent::BuyToOpen,
        (OrderSide::Sell, ExecutionAction::Open) => PositionIntent::SellToOpen,
        (OrderSide::Buy, ExecutionAction::Close) => PositionIntent::BuyToClose,
        (OrderSide::Sell, ExecutionAction::Close) => PositionIntent::SellToClose,
    }
}

fn format_snapshot_number(value: Option<f64>, name: &str) -> OptionResult<String> {
    let rounded = round_price(quote_value(value, name)?)?;
    Ok(rounded.to_string())
}

fn execution_snapshot(
    snapshot: Option<&OptionSnapshot>,
) -> OptionResult<Option<ExecutionSnapshot>> {
    Ok(snapshot.map(ExecutionSnapshot::from))
}

fn execution_leg(
    symbol: String,
    leg_type: String,
    quantity: u32,
    side: OrderSide,
    action: &ExecutionAction,
    snapshot: Option<ExecutionSnapshot>,
) -> ExecutionLeg {
    ExecutionLeg {
        symbol,
        ratio_qty: quantity.to_string(),
        position_intent: position_intent_for(&side, action),
        side,
        leg_type,
        snapshot,
    }
}

fn normalize_leg_type(leg_type: &str) -> Option<String> {
    let normalized = leg_type.trim().to_ascii_lowercase();
    let canonical = match normalized.as_str() {
        "longcall" | "longcall_low" | "longcall_high" | "bwb_longcall_low"
        | "bwb_longcall_high" => "longcall",
        "shortcall" | "bwb_shortcall" => "shortcall",
        "longput" | "diagonal_longput" => "longput",
        "shortput" | "diagonal_shortput" => "shortput",
        _ => return None,
    };
    Some(canonical.to_string())
}

fn normalize_execution_side(side: &str) -> Option<OrderSide> {
    OrderSide::from_str(side).ok()
}

fn normalize_position_intent(position_intent: &str) -> Option<PositionIntent> {
    PositionIntent::from_str(position_intent).ok()
}

fn leg_side_from_type(leg_type: &str, action: &ExecutionAction) -> Option<OrderSide> {
    let normalized = normalize_leg_type(leg_type)?;
    let is_long = normalized.starts_with("long");
    Some(match (is_long, action) {
        (true, ExecutionAction::Open) => OrderSide::Buy,
        (false, ExecutionAction::Open) => OrderSide::Sell,
        (true, ExecutionAction::Close) => OrderSide::Sell,
        (false, ExecutionAction::Close) => OrderSide::Buy,
    })
}

pub fn leg_type(
    symbol: &str,
    side: &str,
    position_intent: &str,
    explicit_leg_type: Option<&str>,
) -> Option<String> {
    if let Some(explicit_leg_type) = explicit_leg_type.and_then(normalize_leg_type) {
        return Some(explicit_leg_type);
    }

    let side = normalize_execution_side(side)?;
    let position_intent = normalize_position_intent(position_intent)?;
    let parsed = contract::parse_occ_symbol(symbol)?;
    let is_close = position_intent.is_close();
    let is_long = match side {
        OrderSide::Buy => !is_close,
        OrderSide::Sell => is_close,
    };

    Some(format!(
        "{}{}",
        if is_long { "long" } else { "short" },
        parsed.option_right.as_str()
    ))
}

fn normalize_roll_quantity(qty: Option<i64>) -> u32 {
    match qty {
        Some(value) if value.is_positive() => value.unsigned_abs() as u32,
        _ => 1,
    }
}

pub fn roll_request(
    current_contract: &str,
    target_contract: Option<&str>,
    new_strike: Option<f64>,
    new_expiration: Option<&str>,
    leg_type: Option<&str>,
    qty: Option<i64>,
) -> Option<RollRequest> {
    let current_contract = current_contract.trim();
    if current_contract.is_empty() {
        return None;
    }

    let leg_type = match leg_type.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Some(normalize_leg_type(value)?),
        None => None,
    };

    let (new_strike, new_expiration) = match target_contract
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(target_contract) => {
            let parsed = contract::parse_occ_symbol(target_contract)?;
            (Some(parsed.strike), parsed.expiration_date)
        }
        None => {
            let new_strike = new_strike.filter(|value| value.is_finite())?;
            let new_expiration = clock::parse_date(new_expiration?.trim()).ok()?;
            (Some(new_strike), new_expiration)
        }
    };

    Some(RollRequest {
        current_contract: current_contract.to_string(),
        leg_type,
        qty: normalize_roll_quantity(qty),
        new_strike,
        new_expiration,
    })
}

fn direct_quote(input: &ExecutionLegInput) -> OptionQuote {
    let bid = input.bid;
    let ask = input.ask;
    let price = input.price;

    if bid.is_none() && ask.is_none() && price.is_none() {
        return OptionQuote {
            bid: None,
            ask: None,
            mark: None,
            last: None,
        };
    }

    if bid.is_none() && ask.is_none() {
        let price = price.unwrap_or(0.0);
        if let Some(spread_percent) = input
            .spread_percent
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            let spread = (price * spread_percent).max(0.0);
            return quote(&OptionQuote {
                bid: Some(price - spread / 2.0),
                ask: Some(price + spread / 2.0),
                mark: Some(price),
                last: Some(price),
            });
        }

        return quote(&OptionQuote {
            bid: Some(price),
            ask: Some(price),
            mark: Some(price),
            last: Some(price),
        });
    }

    quote(&OptionQuote {
        bid,
        ask,
        mark: price,
        last: price,
    })
}

fn direct_execution_snapshot(input: &ExecutionLegInput) -> OptionResult<Option<ExecutionSnapshot>> {
    if let Some(snapshot) = &input.snapshot {
        return Ok(Some(snapshot.clone()));
    }

    let normalized_quote = direct_quote(input);
    if normalized_quote.bid.is_none()
        && normalized_quote.ask.is_none()
        && normalized_quote.mark.is_none()
    {
        return Ok(None);
    }

    Ok(Some(ExecutionSnapshot {
        contract: input.contract.clone(),
        timestamp: input.timestamp.clone().unwrap_or_default(),
        bid: format_snapshot_number(normalized_quote.bid, "bid")?,
        ask: format_snapshot_number(normalized_quote.ask, "ask")?,
        price: format_snapshot_number(normalized_quote.mark.or(normalized_quote.last), "price")?,
        greeks: input
            .greeks
            .as_ref()
            .map(|greeks| Greeks {
                delta: greeks.delta.unwrap_or(0.0),
                gamma: greeks.gamma.unwrap_or(0.0),
                vega: greeks.vega.unwrap_or(0.0),
                theta: greeks.theta.unwrap_or(0.0),
                rho: greeks.rho.unwrap_or(0.0),
            })
            .unwrap_or(Greeks {
                delta: 0.0,
                gamma: 0.0,
                vega: 0.0,
                theta: 0.0,
                rho: 0.0,
            }),
        iv: input.iv.unwrap_or(0.0),
    }))
}

pub fn leg(input: ExecutionLegInput) -> Option<ExecutionLeg> {
    let contract_info = contract::parse_occ_symbol(&input.contract)?;
    let leg_type = normalize_leg_type(&input.leg_type)?;
    if !leg_type.ends_with(contract_info.option_right.as_str()) {
        return None;
    }
    let side = leg_side_from_type(&leg_type, &input.action)?;

    Some(execution_leg(
        input.contract.clone(),
        leg_type,
        input.quantity.unwrap_or(1).max(1),
        side,
        &input.action,
        direct_execution_snapshot(&input).ok()?,
    ))
}

fn normalized_selection_quantity(quantity: Option<u32>, position_quantity: u32) -> u32 {
    match quantity {
        Some(value) if value > 0 => value.min(position_quantity.max(1)),
        _ => position_quantity.max(1),
    }
}

fn range_from_positions(positions: &[OptionPosition]) -> OptionResult<ExecutionQuoteRange> {
    let mut best = 0.0;
    let mut worst = 0.0;

    for position in positions {
        let Some(snapshot) = position.snapshot_ref() else {
            continue;
        };
        let (bid, ask) = quote_bid_ask(&snapshot.quote)?;
        let quantity = position.quantity() as f64;
        match position.position_side() {
            PositionSide::Long => {
                best += bid * quantity;
                worst += ask * quantity;
            }
            PositionSide::Short => {
                best -= ask * quantity;
                worst -= bid * quantity;
            }
        }
    }

    Ok(ExecutionQuoteRange {
        best_price: round_price(best)?,
        worst_price: round_price(worst)?,
    })
}

fn range_from_legs(legs: &[QuotedLeg]) -> OptionResult<ExecutionQuoteRange> {
    let mut best = 0.0;
    let mut worst = 0.0;

    for leg in legs {
        let (bid, ask) = quote_bid_ask(&leg.quote)?;
        let quantity = leg.ratio_quantity as f64;
        match leg.order_side {
            OrderSide::Buy => {
                best += bid * quantity;
                worst += ask * quantity;
            }
            OrderSide::Sell => {
                best -= ask * quantity;
                worst -= bid * quantity;
            }
        }
    }

    Ok(ExecutionQuoteRange {
        best_price: round_price(best)?,
        worst_price: round_price(worst)?,
    })
}

pub fn order_legs(
    positions: &[OptionPosition],
    action: &str,
    include_leg_types: Option<&[String]>,
    exclude_leg_types: Option<&[String]>,
) -> OptionResult<Vec<ExecutionLeg>> {
    let action = ExecutionAction::from_str(action)?;
    let include_leg_types = normalized_filter_set(include_leg_types);
    let exclude_leg_types = normalized_filter_set(exclude_leg_types);
    let mut legs = Vec::new();

    for position in positions {
        if position.quantity() == 0 {
            continue;
        }

        let leg_type = derived_leg_type(position);
        let normalized_leg_type = leg_type.to_ascii_lowercase();
        if !include_leg_types.is_empty() && !include_leg_types.contains(&normalized_leg_type) {
            continue;
        }
        if exclude_leg_types.contains(&normalized_leg_type) {
            continue;
        }

        let side = order_side_for_action(&position.position_side(), &action);
        legs.push(execution_leg(
            position.occ_symbol().to_string(),
            leg_type,
            position.quantity(),
            side,
            &action,
            execution_snapshot(position.snapshot_ref())?,
        ));
    }

    Ok(legs)
}

pub fn roll_legs(
    positions: &[OptionPosition],
    snapshots: &HashMap<String, ExecutionSnapshot>,
    selections: &[RollLegSelection],
) -> OptionResult<Vec<ExecutionLeg>> {
    let mut positions_by_leg_type = HashMap::new();
    for position in positions {
        positions_by_leg_type.insert(derived_leg_type(position).to_ascii_lowercase(), position);
    }

    let mut snapshots_by_leg_type = HashMap::new();
    for (leg_type, snapshot) in snapshots {
        snapshots_by_leg_type.insert(leg_type.trim().to_ascii_lowercase(), snapshot.clone());
    }

    let mut legs = Vec::new();
    for selection in selections {
        let normalized_leg_type = selection.leg_type.trim().to_ascii_lowercase();
        let Some(position) = positions_by_leg_type.get(&normalized_leg_type) else {
            continue;
        };
        let Some(snapshot) = snapshots_by_leg_type.get(&normalized_leg_type) else {
            continue;
        };

        let quantity = normalized_selection_quantity(selection.quantity, position.quantity());
        let close_side = order_side_for_action(&position.position_side(), &ExecutionAction::Close);
        legs.push(execution_leg(
            position.occ_symbol().to_string(),
            normalized_leg_type.clone(),
            quantity,
            close_side,
            &ExecutionAction::Close,
            execution_snapshot(position.snapshot_ref())?,
        ));

        let open_side = order_side_for_action(&position.position_side(), &ExecutionAction::Open);
        legs.push(execution_leg(
            snapshot.contract.clone(),
            normalized_leg_type,
            quantity,
            open_side,
            &ExecutionAction::Open,
            Some(snapshot.clone()),
        ));
    }

    Ok(legs)
}

pub fn best_worst(
    source: &(impl QuoteRangeLike + ?Sized),
    structure_quantity: Option<u32>,
) -> OptionResult<ScaledExecutionQuoteRange> {
    let per_structure = source.quote_range()?;
    scale_quote_range(
        per_structure.best_price,
        per_structure.worst_price,
        structure_quantity.unwrap_or(1),
    )
}

pub fn scale_quote(price: f64, structure_quantity: u32) -> OptionResult<ScaledExecutionQuote> {
    ensure_finite("price", price)?;
    let structure_quantity_f64 = structure_quantity as f64;
    let normalized_price = round_price(price)?;
    let total_price = round_price(normalized_price * structure_quantity_f64)?;

    Ok(ScaledExecutionQuote {
        structure_quantity,
        price: normalized_price,
        total_price,
        total_dollars: round_price(total_price * CONTRACT_MULTIPLIER)?,
    })
}

pub fn scale_quote_range(
    best_price: f64,
    worst_price: f64,
    structure_quantity: u32,
) -> OptionResult<ScaledExecutionQuoteRange> {
    ensure_finite("best_price", best_price)?;
    ensure_finite("worst_price", worst_price)?;
    let structure_quantity_f64 = structure_quantity as f64;
    let per_structure = ExecutionQuoteRange {
        best_price: round_price(best_price)?,
        worst_price: round_price(worst_price)?,
    };
    let per_order = ExecutionQuoteRange {
        best_price: round_price(per_structure.best_price * structure_quantity_f64)?,
        worst_price: round_price(per_structure.worst_price * structure_quantity_f64)?,
    };
    let dollars = ExecutionQuoteRange {
        best_price: round_price(per_order.best_price * CONTRACT_MULTIPLIER)?,
        worst_price: round_price(per_order.worst_price * CONTRACT_MULTIPLIER)?,
    };

    Ok(ScaledExecutionQuoteRange {
        structure_quantity,
        per_structure,
        per_order,
        dollars,
    })
}

pub fn limit_quote_by_progress(
    best_price: f64,
    worst_price: f64,
    progress: f64,
) -> OptionResult<f64> {
    ensure_finite("best_price", best_price)?;
    ensure_finite("worst_price", worst_price)?;
    let progress = clamp_progress(progress)?;
    round_price(best_price + (worst_price - best_price) * progress)
}

pub fn progress_of_limit(best_price: f64, worst_price: f64, limit_price: f64) -> OptionResult<f64> {
    ensure_finite("best_price", best_price)?;
    ensure_finite("worst_price", worst_price)?;
    ensure_finite("limit_price", limit_price)?;
    if (worst_price - best_price).abs() < 1e-12 {
        return Ok(0.5);
    }
    numeric::round(
        ((limit_price - best_price) / (worst_price - best_price)).clamp(0.0, 1.0),
        12,
    )
    .map_err(|_| {
        OptionError::new(
            "invalid_execution_quote_input",
            "unable to normalize limit progress",
        )
    })
}
