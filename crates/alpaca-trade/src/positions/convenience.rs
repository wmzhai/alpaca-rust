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
