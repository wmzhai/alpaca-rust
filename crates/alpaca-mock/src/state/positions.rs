#![allow(dead_code)]

use std::collections::HashMap;
use std::str::FromStr;

use chrono::NaiveDate;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedPosition {
    pub(crate) symbol: String,
    pub(crate) qty: Decimal,
    pub(crate) avg_entry_price: Decimal,
}

impl PositionBook {
    pub(crate) fn export_signed(&self) -> Vec<SignedPosition> {
        let mut positions = self
            .instrument_positions
            .values()
            .filter(|position| position.net_qty != Decimal::ZERO)
            .map(|position| SignedPosition {
                symbol: position.instrument_identity.symbol.clone(),
                qty: position.net_qty,
                avg_entry_price: position.avg_entry_price(),
            })
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        positions
    }

    pub(crate) fn replace_signed(&mut self, positions: Vec<SignedPosition>) {
        self.instrument_positions.clear();
        self.do_not_exercise_overrides.clear();
        for position in positions {
            if position.qty == Decimal::ZERO || position.symbol.trim().is_empty() {
                continue;
            }
            let asset_class = if parse_option_symbol(&position.symbol).is_some() {
                "us_option"
            } else {
                "us_equity"
            };
            let asset_id = mock_asset_id(&position.symbol);
            let mut held = InstrumentPosition::new(&asset_id, &position.symbol, asset_class);
            let price = if position.avg_entry_price > Decimal::ZERO {
                position.avg_entry_price
            } else {
                Decimal::ONE
            };
            if position.qty > Decimal::ZERO {
                held.open_long(position.qty, price, None, "imported");
            } else {
                held.open_short(position.qty.abs(), price, None, "imported");
            }
            let key = InstrumentKey::new(&asset_id, &position.symbol);
            self.instrument_positions.insert(key, held);
        }
    }

    pub(crate) fn apply_execution(&mut self, execution: &ExecutionFact) {
        self.rekey_symbol_position(&execution.symbol, &execution.asset_id);
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

    // Import and restore used to key positions by the symbol. Fills use mock-asset ids.
    // Net every same-symbol row onto the fill identity before applying the execution.
    fn rekey_symbol_position(&mut self, symbol: &str, asset_id: &str) {
        let canonical = InstrumentKey::new(asset_id, symbol);
        let legacy_keys: Vec<InstrumentKey> = self
            .instrument_positions
            .keys()
            .filter(|key| key.symbol.eq_ignore_ascii_case(symbol) && key.asset_id != asset_id)
            .cloned()
            .collect();
        if legacy_keys.is_empty() {
            return;
        }
        let mut parts = Vec::new();
        if let Some(existing) = self.instrument_positions.remove(&canonical) {
            parts.push(existing);
        }
        for key in legacy_keys {
            if let Some(held) = self.instrument_positions.remove(&key) {
                parts.push(held);
            }
        }
        if let Some(merged) = net_same_symbol(asset_id, parts) {
            self.instrument_positions.insert(canonical, merged);
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
    pub(crate) unrealized_intraday_pl: Decimal,
    #[serde(serialize_with = "alpaca_core::decimal::string_contract::serialize_decimal")]
    pub(crate) unrealized_intraday_plpc: Decimal,
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
    let previous_market_value = (position.net_qty * lastday_price).round_dp(8);
    let unrealized_intraday_pl = (market_value - previous_market_value).round_dp(8);
    let unrealized_intraday_plpc =
        ratio_or_zero(unrealized_intraday_pl, previous_market_value.abs());
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
        unrealized_intraday_pl,
        unrealized_intraday_plpc,
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
    let unrealized_intraday_pl = Decimal::ZERO;
    let unrealized_intraday_plpc = Decimal::ZERO;
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
        unrealized_intraday_pl,
        unrealized_intraday_plpc,
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
    pub(crate) expiration_date: NaiveDate,
    pub(crate) contract_type: OptionContractType,
    pub(crate) strike_price: Decimal,
}

/// Combine split rows for one symbol. The surviving side keeps its own average
/// cost; the opposite side is closed against it and is not blended in.
fn net_same_symbol(asset_id: &str, parts: Vec<InstrumentPosition>) -> Option<InstrumentPosition> {
    let template = parts
        .iter()
        .find(|part| part.instrument_identity.asset_id == asset_id)
        .or_else(|| parts.first())?;
    let mut long_qty = Decimal::ZERO;
    let mut long_cost = Decimal::ZERO;
    let mut short_qty = Decimal::ZERO;
    let mut short_cost = Decimal::ZERO;
    for part in &parts {
        if part.net_qty > Decimal::ZERO {
            long_qty += part.net_qty;
            long_cost += part.net_qty * part.avg_entry_price();
        } else if part.net_qty < Decimal::ZERO {
            let qty = -part.net_qty;
            short_qty += qty;
            short_cost += qty * part.avg_entry_price();
        }
    }
    let net = long_qty - short_qty;
    if net == Decimal::ZERO {
        return None;
    }
    let symbol = template.instrument_identity.symbol.clone();
    let asset_class = template.instrument_identity.asset_class.clone();
    let snapshot = template.market_snapshot.clone();
    let occurred_at = template.last_update_at.clone();
    let mut merged = InstrumentPosition::new(asset_id, &symbol, &asset_class);
    if net > Decimal::ZERO {
        let price = (long_cost / long_qty).round_dp(8);
        merged.open_long(net, price, snapshot.as_ref(), &occurred_at);
    } else {
        let price = (short_cost / short_qty).round_dp(8);
        merged.open_short(-net, price, snapshot.as_ref(), &occurred_at);
    }
    Some(merged)
}

pub(crate) fn mock_asset_id(symbol: &str) -> String {
    let mut sanitized = String::with_capacity(symbol.len());
    for ch in symbol.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }
    format!("mock-asset-{}", sanitized.trim_matches('-'))
}

pub(crate) fn parse_option_symbol(symbol: &str) -> Option<ParsedOptionSymbol> {
    let root_len = symbol.len().checked_sub(15)?;
    let underlying_symbol = symbol.get(..root_len)?.trim().to_owned();
    if underlying_symbol.is_empty() {
        return None;
    }

    let expiration = symbol.get(root_len..root_len + 6)?;
    let year = 2000 + expiration.get(..2)?.parse::<i32>().ok()?;
    let month = expiration.get(2..4)?.parse::<u32>().ok()?;
    let day = expiration.get(4..6)?.parse::<u32>().ok()?;
    let expiration_date = NaiveDate::from_ymd_opt(year, month, day)?;
    let contract_type = match symbol.get(root_len + 6..root_len + 7)? {
        "C" => OptionContractType::Call,
        "P" => OptionContractType::Put,
        _ => return None,
    };
    let strike = symbol.get(root_len + 7..)?;
    let strike_price = Decimal::from_str(strike).ok()? / Decimal::new(1000, 0);

    Some(ParsedOptionSymbol {
        underlying_symbol,
        expiration_date,
        contract_type,
        strike_price,
    })
}

#[cfg(test)]
mod tests {
    use alpaca_trade::orders::{OrderSide, PositionIntent};
    use rust_decimal::Decimal;

    use super::super::executions::ExecutionFact;
    use super::{mock_asset_id, InstrumentKey, InstrumentPosition, PositionBook, SignedPosition};

    #[test]
    fn signed_roundtrip_keeps_long_and_short() {
        let mut book = PositionBook::default();
        book.replace_signed(vec![
            SignedPosition {
                symbol: "QQQ270617C00600000".to_owned(),
                qty: Decimal::ONE,
                avg_entry_price: Decimal::new(12565, 2),
            },
            SignedPosition {
                symbol: "QQQ261120C00679000".to_owned(),
                qty: -Decimal::ONE,
                avg_entry_price: Decimal::new(2605, 2),
            },
        ]);
        let exported = book.export_signed();
        assert_eq!(exported.len(), 2);
        assert_eq!(exported.iter().find(|position| position.symbol.ends_with("C00600000")).unwrap().qty, Decimal::ONE);
        assert_eq!(exported.iter().find(|position| position.symbol.ends_with("C00679000")).unwrap().qty, -Decimal::ONE);
    }

    #[test]
    fn imported_and_filled_positions_share_one_asset_id() {
        let symbol = "QQQ270617C00600000";
        let asset_id = mock_asset_id(symbol);
        let mut book = PositionBook::default();
        book.replace_signed(vec![SignedPosition {
            symbol: symbol.to_owned(),
            qty: Decimal::from(2),
            avg_entry_price: Decimal::ONE,
        }]);
        book.apply_execution(&execution(asset_id.clone(), symbol, OrderSide::Sell, PositionIntent::SellToClose, Decimal::ONE));
        let open = book.list_open_positions();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].instrument_identity.asset_id, asset_id);
        assert_eq!(open[0].net_qty, Decimal::ONE);

        book.apply_execution(&execution(asset_id.clone(), symbol, OrderSide::Buy, PositionIntent::BuyToOpen, Decimal::ONE));
        let open = book.list_open_positions();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].net_qty, Decimal::from(2));

        let mut legacy = PositionBook::default();
        let mut held = InstrumentPosition::new(symbol, symbol, "us_option");
        held.open_long(Decimal::from(2), Decimal::ONE, None, "imported");
        legacy
            .instrument_positions
            .insert(InstrumentKey::new(symbol, symbol), held);
        legacy.apply_execution(&execution(asset_id.clone(), symbol, OrderSide::Sell, PositionIntent::SellToClose, Decimal::ONE));
        let open = legacy.list_open_positions();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].instrument_identity.asset_id, asset_id);
        assert_eq!(open[0].net_qty, Decimal::ONE);
    }

    #[test]
    fn opposite_rows_net_before_the_next_fill() {
        let symbol = "AAPL";
        let asset_id = mock_asset_id(symbol);
        let mut book = PositionBook::default();
        let mut canonical = InstrumentPosition::new(&asset_id, symbol, "us_equity");
        canonical.open_short(Decimal::from(10), Decimal::from(30), None, "canonical");
        book.instrument_positions
            .insert(InstrumentKey::new(&asset_id, symbol), canonical);
        let mut legacy = InstrumentPosition::new(symbol, symbol, "us_equity");
        legacy.open_long(Decimal::from(100), Decimal::from(50), None, "legacy");
        book.instrument_positions
            .insert(InstrumentKey::new(symbol, symbol), legacy);

        book.apply_execution(&ExecutionFact::new(
            1,
            "order-1".to_owned(),
            asset_id.clone(),
            symbol.to_owned(),
            "us_equity".to_owned(),
            OrderSide::Sell,
            None,
            Decimal::from(10),
            Decimal::from(55),
            None,
            "2026-11-01T15:00:00Z".to_owned(),
        ));

        let open = book.list_open_positions();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].instrument_identity.asset_id, asset_id);
        assert_eq!(open[0].net_qty, Decimal::from(80));
        assert_eq!(open[0].avg_entry_price(), Decimal::from(50));
    }

    fn execution(asset_id: String, symbol: &str, side: OrderSide, intent: PositionIntent, qty: Decimal) -> ExecutionFact {
        ExecutionFact::new(
            1,
            "order-1".to_owned(),
            asset_id,
            symbol.to_owned(),
            "us_option".to_owned(),
            side,
            Some(intent),
            qty,
            Decimal::ONE,
            None,
            "2026-11-01T15:00:00Z".to_owned(),
        )
    }
}
