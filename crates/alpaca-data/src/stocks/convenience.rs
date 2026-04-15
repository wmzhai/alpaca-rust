use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{Snapshot, SnapshotResponse};

fn timestamp_parts<'a>(
    latest_trade: Option<&'a str>,
    latest_quote: Option<&'a str>,
    minute_bar: Option<&'a str>,
    daily_bar: Option<&'a str>,
    prev_daily_bar: Option<&'a str>,
) -> Option<&'a str> {
    [
        latest_trade,
        latest_quote,
        minute_bar,
        daily_bar,
        prev_daily_bar,
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.trim().is_empty())
    .max()
}

fn price_parts(
    latest_trade: Option<Decimal>,
    bid: Option<Decimal>,
    ask: Option<Decimal>,
    minute_close: Option<Decimal>,
    daily_close: Option<Decimal>,
    prev_daily_close: Option<Decimal>,
) -> Option<Decimal> {
    latest_trade
        .or_else(|| match (bid, ask) {
            (Some(bid), Some(ask)) => Some((bid + ask) / Decimal::from(2u8)),
            (Some(bid), None) => Some(bid),
            (None, Some(ask)) => Some(ask),
            (None, None) => None,
        })
        .or(minute_close)
        .or(daily_close)
        .or(prev_daily_close)
}

impl Snapshot {
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        timestamp_parts(
            self.latest_trade.as_ref().and_then(|trade| trade.t.as_deref()),
            self.latest_quote.as_ref().and_then(|quote| quote.t.as_deref()),
            self.minute_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.daily_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.prev_daily_bar
                .as_ref()
                .and_then(|bar| bar.t.as_deref()),
        )
    }

    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        price_parts(
            self.latest_trade.as_ref().and_then(|trade| trade.p),
            self.latest_quote.as_ref().and_then(|quote| quote.bp),
            self.latest_quote.as_ref().and_then(|quote| quote.ap),
            self.minute_bar.as_ref().and_then(|bar| bar.c),
            self.daily_bar.as_ref().and_then(|bar| bar.c),
            self.prev_daily_bar.as_ref().and_then(|bar| bar.c),
        )
    }
}

impl SnapshotResponse {
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        timestamp_parts(
            self.latest_trade.as_ref().and_then(|trade| trade.t.as_deref()),
            self.latest_quote.as_ref().and_then(|quote| quote.t.as_deref()),
            self.minute_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.daily_bar.as_ref().and_then(|bar| bar.t.as_deref()),
            self.prev_daily_bar
                .as_ref()
                .and_then(|bar| bar.t.as_deref()),
        )
    }

    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        price_parts(
            self.latest_trade.as_ref().and_then(|trade| trade.p),
            self.latest_quote.as_ref().and_then(|quote| quote.bp),
            self.latest_quote.as_ref().and_then(|quote| quote.ap),
            self.minute_bar.as_ref().and_then(|bar| bar.c),
            self.daily_bar.as_ref().and_then(|bar| bar.c),
            self.prev_daily_bar.as_ref().and_then(|bar| bar.c),
        )
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
