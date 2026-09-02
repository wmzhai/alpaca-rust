use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{Location, Snapshot};

impl Snapshot {
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        [
            self.latest_trade
                .as_ref()
                .and_then(|trade| trade.t.as_deref()),
            self.latest_quote
                .as_ref()
                .and_then(|quote| quote.t.as_deref()),
            self.minute_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.daily_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.prev_daily_bar
                .as_ref()
                .and_then(|bar| bar.t.as_deref()),
        ]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .max()
    }

    #[must_use]
    pub fn bid_price(&self) -> Option<Decimal> {
        self.latest_quote.as_ref().and_then(|quote| quote.bp)
    }

    #[must_use]
    pub fn ask_price(&self) -> Option<Decimal> {
        self.latest_quote.as_ref().and_then(|quote| quote.ap)
    }

    #[must_use]
    pub fn last_price(&self) -> Option<Decimal> {
        self.latest_trade.as_ref().and_then(|trade| trade.p)
    }

    #[must_use]
    pub fn mark_price(&self) -> Option<Decimal> {
        match (self.bid_price(), self.ask_price()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / Decimal::from(2u8)),
            (Some(bid), None) => Some(bid),
            (None, Some(ask)) => Some(ask),
            (None, None) => None,
        }
    }

    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.last_price().or_else(|| self.mark_price())
    }
}

#[must_use]
pub fn ordered_snapshots(snapshots: &HashMap<String, Snapshot>) -> Vec<(&str, &Snapshot)> {
    let mut symbols = snapshots.keys().map(String::as_str).collect::<Vec<_>>();
    symbols.sort_unstable();
    symbols
        .into_iter()
        .filter_map(|symbol| {
            snapshots
                .get_key_value(symbol)
                .map(|(symbol, snapshot)| (symbol.as_str(), snapshot))
        })
        .collect()
}

#[must_use]
pub fn preferred_location() -> Location {
    Location::Us
}
