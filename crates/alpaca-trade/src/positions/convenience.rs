use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;

use crate::positions::Position;

#[must_use]
pub fn option_qty_map(positions: &[Position]) -> HashMap<String, i32> {
    let mut mapped = HashMap::new();

    for position in positions {
        let contract = position.symbol.trim();
        if contract.len() <= 10 {
            continue;
        }

        mapped.insert(
            contract.to_string(),
            position.qty.trunc().to_i32().unwrap_or(0),
        );
    }

    mapped
}

#[must_use]
pub fn structure_quantity<'a>(
    template_positions: impl IntoIterator<Item = (&'a str, i32)>,
    live_positions: &HashMap<String, i32>,
) -> Option<i32> {
    let mut structure_qty: Option<i32> = None;

    for (symbol, template_qty) in template_positions
        .into_iter()
        .filter(|(_, signed_qty)| *signed_qty != 0)
    {
        let live_qty = live_positions.get(symbol).copied().unwrap_or(0);
        if live_qty == 0 {
            continue;
        }

        if live_qty.signum() != template_qty.signum() {
            return None;
        }

        let template_abs = template_qty.abs();
        if template_abs == 0 {
            continue;
        }

        let live_abs = live_qty.abs();
        if live_abs % template_abs != 0 {
            return None;
        }

        let resolved_qty = live_abs / template_abs;
        if resolved_qty <= 0 {
            return None;
        }

        if let Some(current_qty) = structure_qty {
            if current_qty != resolved_qty {
                return None;
            }
        } else {
            structure_qty = Some(resolved_qty);
        }
    }

    structure_qty
}

pub fn reconcile_signed_positions<T>(
    positions: &mut Vec<T>,
    live_positions: &HashMap<String, i32>,
    symbol: impl Fn(&T) -> &str + Copy,
    mut set_signed_qty: impl FnMut(&mut T, i32),
) {
    for position in positions.iter_mut() {
        let live_qty = live_positions.get(symbol(position)).copied().unwrap_or(0);
        set_signed_qty(position, live_qty);
    }
    positions.retain(|position| live_positions.get(symbol(position)).copied().unwrap_or(0) != 0);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{reconcile_signed_positions, structure_quantity};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct PositionRow {
        contract: String,
        qty: i32,
    }

    #[test]
    fn structure_quantity_requires_consistent_signed_ratio() {
        let mut live_positions = HashMap::new();
        live_positions.insert("SPY260417C00550000".to_string(), 2);
        live_positions.insert("SPY260417P00500000".to_string(), -2);

        let quantity = structure_quantity(
            [("SPY260417C00550000", 1), ("SPY260417P00500000", -1)],
            &live_positions,
        );

        assert_eq!(quantity, Some(2));
    }

    #[test]
    fn reconcile_signed_positions_updates_qty_and_drops_missing_rows() {
        let mut rows = vec![
            PositionRow {
                contract: "SPY260417C00550000".to_string(),
                qty: 1,
            },
            PositionRow {
                contract: "SPY260417P00500000".to_string(),
                qty: -1,
            },
        ];

        let mut live_positions = HashMap::new();
        live_positions.insert("SPY260417C00550000".to_string(), 3);

        reconcile_signed_positions(
            &mut rows,
            &live_positions,
            |row| row.contract.as_str(),
            |row, qty| row.qty = qty,
        );

        assert_eq!(
            rows,
            vec![PositionRow {
                contract: "SPY260417C00550000".to_string(),
                qty: 3,
            }]
        );
    }
}
