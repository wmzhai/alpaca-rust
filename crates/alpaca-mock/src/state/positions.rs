#![allow(dead_code)]

use std::collections::HashMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Serialize;

use alpaca_trade::orders::{OrderSide, PositionIntent};

use super::ExecutionFact;
use super::market_data::InstrumentSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InstrumentKey {
    asset_id: String,
    symbol: String,
}

impl InstrumentKey {
    fn new(asset_id: &str, symbol: &str) -> Self {
        Self {
            asset_id: asset_id.to_owned(),
            symbol: symbol.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstrumentIdentity {
    pub(crate) asset_id: String,
    pub(crate) symbol: String,
    pub(crate) exchange: String,
    pub(crate) asset_class: String,
    pub(crate) asset_marginable: bool,
}

impl InstrumentIdentity {
    fn new(asset_id: &str, symbol: &str, asset_class: &str) -> Self {
        Self {
            asset_id: asset_id.to_owned(),
            symbol: symbol.to_owned(),
            exchange: String::new(),
            asset_class: asset_class.to_owned(),
            asset_marginable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PositionSide {
    Long,
    Short,
}

impl PositionSide {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenLot {
    pub(crate) side: PositionSide,
    pub(crate) qty: Decimal,
    pub(crate) avg_entry_price: Decimal,
    pub(crate) opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstrumentPosition {
    pub(crate) instrument_identity: InstrumentIdentity,
    pub(crate) open_lots: Vec<OpenLot>,
    pub(crate) net_qty: Decimal,
    pub(crate) market_snapshot: Option<InstrumentSnapshot>,
    pub(crate) last_update_at: String,
}

impl InstrumentPosition {
    fn new(asset_id: &str, symbol: &str, asset_class: &str) -> Self {
        Self {
            instrument_identity: InstrumentIdentity::new(asset_id, symbol, asset_class),
            open_lots: Vec::new(),
            net_qty: Decimal::ZERO,
            market_snapshot: None,
            last_update_at: String::new(),
        }
    }

    pub(crate) fn avg_entry_price(&self) -> Decimal {
        self.open_lots
            .first()
            .map(|lot| lot.avg_entry_price)
            .unwrap_or(Decimal::ZERO)
    }

    fn open_long(
        &mut self,
        qty: Decimal,
        price: Decimal,
        market_snapshot: Option<&InstrumentSnapshot>,
        occurred_at: &str,
    ) {
        if qty <= Decimal::ZERO {
            return;
        }
        let existing_qty = if self.net_qty > Decimal::ZERO {
            self.net_qty
        } else {
            Decimal::ZERO
        };
        let existing_cost = existing_qty * self.avg_entry_price();
        let next_qty = existing_qty + qty;
        let next_avg = if next_qty == Decimal::ZERO {
            Decimal::ZERO
        } else {
            ((existing_cost + (qty * price)) / next_qty).round_dp(8)
        };
        self.open_lots = vec![OpenLot {
            side: PositionSide::Long,
            qty: next_qty,
            avg_entry_price: next_avg,
            opened_at: occurred_at.to_owned(),
        }];
        self.net_qty = next_qty;
        self.market_snapshot = market_snapshot.cloned();
        self.last_update_at = occurred_at.to_owned();
    }

    fn close_long(&mut self, qty: Decimal, occurred_at: &str) {
        if qty <= Decimal::ZERO || self.net_qty <= Decimal::ZERO {
            return;
        }
        let current_qty = self.net_qty;
        let remaining_qty = if current_qty > qty {
            current_qty - qty
        } else {
            Decimal::ZERO
        };
        if remaining_qty == Decimal::ZERO {
            self.open_lots.clear();
            self.net_qty = Decimal::ZERO;
        } else {
            if let Some(lot) = self.open_lots.first_mut() {
                lot.qty = remaining_qty;
            }
            self.net_qty = remaining_qty;
        }
        self.last_update_at = occurred_at.to_owned();
    }

    fn open_short(
        &mut self,
        qty: Decimal,
        price: Decimal,
        market_snapshot: Option<&InstrumentSnapshot>,
        occurred_at: &str,
    ) {
        if qty <= Decimal::ZERO {
            return;
        }
        let existing_qty = if self.net_qty < Decimal::ZERO {
            -self.net_qty
        } else {
            Decimal::ZERO
        };
        let existing_cost = existing_qty * self.avg_entry_price();
        let next_qty = existing_qty + qty;
        let next_avg = if next_qty == Decimal::ZERO {
            Decimal::ZERO
        } else {
            ((existing_cost + (qty * price)) / next_qty).round_dp(8)
        };
        self.open_lots = vec![OpenLot {
            side: PositionSide::Short,
            qty: next_qty,
            avg_entry_price: next_avg,
            opened_at: occurred_at.to_owned(),
        }];
        self.net_qty = -next_qty;
        self.market_snapshot = market_snapshot.cloned();
        self.last_update_at = occurred_at.to_owned();
    }

    fn close_short(&mut self, qty: Decimal, occurred_at: &str) {
        if qty <= Decimal::ZERO || self.net_qty >= Decimal::ZERO {
            return;
        }
        let current_qty = -self.net_qty;
        let remaining_qty = if current_qty > qty {
            current_qty - qty
        } else {
            Decimal::ZERO
        };
        if remaining_qty == Decimal::ZERO {
            self.open_lots.clear();
            self.net_qty = Decimal::ZERO;
        } else {
            if let Some(lot) = self.open_lots.first_mut() {
                lot.qty = remaining_qty;
            }
            self.net_qty = -remaining_qty;
        }
        self.last_update_at = occurred_at.to_owned();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PositionBook {
    instrument_positions: HashMap<InstrumentKey, InstrumentPosition>,
    do_not_exercise_overrides: HashMap<String, String>,
}

impl PositionBook {
    pub(crate) fn apply_execution(&mut self, execution: &ExecutionFact) {
        let key = InstrumentKey::new(&execution.asset_id, &execution.symbol);
        let should_remove = {
            let position = self
                .instrument_positions
                .entry(key.clone())
                .or_insert_with(|| {
                    InstrumentPosition::new(
                        &execution.asset_id,
                        &execution.symbol,
                        &execution.asset_class,
                    )
                });

            match execution.position_intent {
                Some(PositionIntent::BuyToOpen) => position.open_long(
                    execution.qty,
                    execution.price,
                    execution.market_snapshot.as_ref(),
                    &execution.occurred_at,
                ),
                Some(PositionIntent::SellToClose) => {
                    position.close_long(execution.qty, &execution.occurred_at)
                }
                Some(PositionIntent::SellToOpen) => position.open_short(
                    execution.qty,
                    execution.price,
                    execution.market_snapshot.as_ref(),
                    &execution.occurred_at,
                ),
                Some(PositionIntent::BuyToClose) => {
                    position.close_short(execution.qty, &execution.occurred_at)
                }
                None => match execution.side {
                    OrderSide::Buy => {
                        if position.net_qty < Decimal::ZERO {
                            let cover_qty = execution.qty.min(-position.net_qty);
                            position.close_short(cover_qty, &execution.occurred_at);
                            let open_qty = execution.qty - cover_qty;
                            position.open_long(
                                open_qty,
                                execution.price,
                                execution.market_snapshot.as_ref(),
                                &execution.occurred_at,
                            );
                        } else {
                            position.open_long(
                                execution.qty,
                                execution.price,
                                execution.market_snapshot.as_ref(),
                                &execution.occurred_at,
                            );
                        }
                    }
                    OrderSide::Sell => {
                        if position.net_qty > Decimal::ZERO {
                            let close_qty = execution.qty.min(position.net_qty);
                            position.close_long(close_qty, &execution.occurred_at);
                            let open_qty = execution.qty - close_qty;
                            position.open_short(
                                open_qty,
                                execution.price,
                                execution.market_snapshot.as_ref(),
                                &execution.occurred_at,
                            );
                        } else {
                            position.open_short(
                                execution.qty,
                                execution.price,
                                execution.market_snapshot.as_ref(),
                                &execution.occurred_at,
                            );
                        }
                    }
                    OrderSide::Unspecified => {}
                },
            }

            position.net_qty == Decimal::ZERO
        };

        if should_remove {
            self.do_not_exercise_overrides.remove(&execution.symbol);
            self.instrument_positions.remove(&key);
        }
    }

    pub(crate) fn list_open_positions(&self) -> Vec<InstrumentPosition> {
        let mut positions = self
            .instrument_positions
            .values()
            .filter(|position| position.net_qty != Decimal::ZERO)
            .cloned()
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| {
            left.instrument_identity
                .symbol
                .cmp(&right.instrument_identity.symbol)
        });
        positions
    }

    pub(crate) fn find_open_position(
        &self,
        symbol_or_asset_id: &str,
    ) -> Option<InstrumentPosition> {
        self.instrument_positions.values().find_map(|position| {
            if position.net_qty == Decimal::ZERO {
                return None;
            }
            if position.instrument_identity.symbol == symbol_or_asset_id
                || position.instrument_identity.asset_id == symbol_or_asset_id
            {
                Some(position.clone())
            } else {
                None
            }
        })
    }

    pub(crate) fn record_do_not_exercise(&mut self, symbol: &str, occurred_at: &str) {
        self.do_not_exercise_overrides
            .insert(symbol.to_owned(), occurred_at.to_owned());
    }

    pub(crate) fn has_do_not_exercise_override(&self, symbol: &str) -> bool {
        self.do_not_exercise_overrides.contains_key(symbol)
    }

    pub(crate) fn clear_do_not_exercise_override(&mut self, symbol: &str) {
        self.do_not_exercise_overrides.remove(symbol);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ProjectedPosition {
    pub(crate) asset_id: String,
    pub(crate) symbol: String,
    pub(crate) exchange: String,
    pub(crate) asset_class: String,
    pub(crate) asset_marginable: bool,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) qty: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) avg_entry_price: Decimal,
    pub(crate) side: String,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) market_value: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) cost_basis: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) unrealized_pl: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) unrealized_plpc: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) current_price: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) lastday_price: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) change_today: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) qty_available: Decimal,
}

pub(crate) fn project_position(
    position: &InstrumentPosition,
    market_snapshot: &InstrumentSnapshot,
) -> ProjectedPosition {
    let qty = position.net_qty;
    let avg_entry_price = position.avg_entry_price();
    let current_price = market_snapshot.mid_price();
    let lastday_price = market_snapshot.previous_close.unwrap_or(current_price);
    let market_value = (position.net_qty * current_price).round_dp(8);
    let cost_basis = (position.net_qty * avg_entry_price).round_dp(8);
    let unrealized_pl = (market_value - cost_basis).round_dp(8);
    let unrealized_plpc = ratio_or_zero(unrealized_pl, cost_basis.abs());
    let change_today = ratio_or_zero(current_price - lastday_price, lastday_price.abs());
    let side = if position.net_qty >= Decimal::ZERO {
        PositionSide::Long
    } else {
        PositionSide::Short
    };

    ProjectedPosition {
        asset_id: position.instrument_identity.asset_id.clone(),
        symbol: position.instrument_identity.symbol.clone(),
        exchange: position.instrument_identity.exchange.clone(),
        asset_class: position.instrument_identity.asset_class.clone(),
        asset_marginable: position.instrument_identity.asset_marginable,
        qty,
        avg_entry_price,
        side: side.as_str().to_owned(),
        market_value,
        cost_basis,
        unrealized_pl,
        unrealized_plpc,
        current_price,
        lastday_price,
        change_today,
        qty_available: qty.abs(),
    }
}

pub(crate) fn project_position_without_market(position: &InstrumentPosition) -> ProjectedPosition {
    if let Some(snapshot) = position.market_snapshot.as_ref() {
        return project_position(position, snapshot);
    }

    let qty = position.net_qty;
    let avg_entry_price = position.avg_entry_price();
    let current_price = avg_entry_price;
    let lastday_price = avg_entry_price;
    let market_value = (position.net_qty * current_price).round_dp(8);
    let cost_basis = (position.net_qty * avg_entry_price).round_dp(8);
    let unrealized_pl = Decimal::ZERO;
    let unrealized_plpc = Decimal::ZERO;
    let change_today = Decimal::ZERO;
    let side = if position.net_qty >= Decimal::ZERO {
        PositionSide::Long
    } else {
        PositionSide::Short
    };

    ProjectedPosition {
        asset_id: position.instrument_identity.asset_id.clone(),
        symbol: position.instrument_identity.symbol.clone(),
        exchange: position.instrument_identity.exchange.clone(),
        asset_class: position.instrument_identity.asset_class.clone(),
        asset_marginable: position.instrument_identity.asset_marginable,
        qty,
        avg_entry_price,
        side: side.as_str().to_owned(),
        market_value,
        cost_basis,
        unrealized_pl,
        unrealized_plpc,
        current_price,
        lastday_price,
        change_today,
        qty_available: qty.abs(),
    }
}

fn ratio_or_zero(numerator: Decimal, denominator: Decimal) -> Decimal {
    if denominator == Decimal::ZERO {
        Decimal::ZERO
    } else {
        (numerator / denominator).round_dp(8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OptionContractType {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedOptionSymbol {
    pub(crate) underlying_symbol: String,
    pub(crate) contract_type: OptionContractType,
    pub(crate) strike_price: Decimal,
}

pub(crate) fn parse_option_symbol(symbol: &str) -> Option<ParsedOptionSymbol> {
    let root_len = symbol.len().checked_sub(15)?;
    let underlying_symbol = symbol.get(..root_len)?.trim().to_owned();
    if underlying_symbol.is_empty() {
        return None;
    }

    let contract_type = match symbol.get(root_len + 6..root_len + 7)? {
        "C" => OptionContractType::Call,
        "P" => OptionContractType::Put,
        _ => return None,
    };
    let strike = symbol.get(root_len + 7..)?;
    let strike_price = Decimal::from_str(strike).ok()? / Decimal::new(1000, 0);

    Some(ParsedOptionSymbol {
        underlying_symbol,
        contract_type,
        strike_price,
    })
}
