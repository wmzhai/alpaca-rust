use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;

use crate::positions::Position;

pub trait SignedPositionLike {
    fn symbol(&self) -> &str;
    fn signed_qty(&self) -> i32;
    fn set_signed_qty(&mut self, qty: i32);
}

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
pub fn structure_quantity<T: SignedPositionLike>(
    template_positions: &[T],
    live_positions: &HashMap<String, i32>,
) -> Option<i32> {
    let mut structure_qty: Option<i32> = None;

    for position in template_positions
        .iter()
        .filter(|position| position.signed_qty() != 0)
    {
        let template_qty = position.signed_qty();
        let live_qty = live_positions.get(position.symbol()).copied().unwrap_or(0);
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

pub fn reconcile_signed_positions<T: SignedPositionLike>(
    positions: &mut Vec<T>,
    live_positions: &HashMap<String, i32>,
) {
    for position in positions.iter_mut() {
        let live_qty = live_positions.get(position.symbol()).copied().unwrap_or(0);
        position.set_signed_qty(live_qty);
    }
    positions.retain(|position| position.signed_qty() != 0);
}
