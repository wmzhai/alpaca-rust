use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{Bar, DataFeed, Snapshot};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BarPoint {
    pub timestamp: String,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: i64,
}

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

fn quote_bid(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.latest_quote.as_ref().and_then(|quote| quote.bp)
}

fn quote_ask(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.latest_quote.as_ref().and_then(|quote| quote.ap)
}

fn session_open(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.daily_bar.as_ref().and_then(|bar| bar.o)
}

fn session_high(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.daily_bar.as_ref().and_then(|bar| bar.h)
}

fn session_low(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.daily_bar.as_ref().and_then(|bar| bar.l)
}

fn session_close(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.daily_bar.as_ref().and_then(|bar| bar.c)
}

fn previous_close(snapshot: &Snapshot) -> Option<Decimal> {
    snapshot.prev_daily_bar.as_ref().and_then(|bar| bar.c)
}

fn session_volume(snapshot: &Snapshot) -> Option<u64> {
    snapshot.daily_bar.as_ref().and_then(|bar| bar.v)
}

impl Snapshot {
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        timestamp_parts(
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
        )
    }

    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        price_parts(
            self.latest_trade.as_ref().and_then(|trade| trade.p),
            self.bid_price(),
            self.ask_price(),
            self.minute_bar.as_ref().and_then(|bar| bar.c),
            self.session_close(),
            self.previous_close(),
        )
    }

    #[must_use]
    pub fn bid_price(&self) -> Option<Decimal> {
        quote_bid(self)
    }

    #[must_use]
    pub fn ask_price(&self) -> Option<Decimal> {
        quote_ask(self)
    }

    #[must_use]
    pub fn session_open(&self) -> Option<Decimal> {
        session_open(self)
    }

    #[must_use]
    pub fn session_high(&self) -> Option<Decimal> {
        session_high(self)
    }

    #[must_use]
    pub fn session_low(&self) -> Option<Decimal> {
        session_low(self)
    }

    #[must_use]
    pub fn session_close(&self) -> Option<Decimal> {
        session_close(self)
    }

    #[must_use]
    pub fn previous_close(&self) -> Option<Decimal> {
        previous_close(self)
    }

    #[must_use]
    pub fn session_volume(&self) -> Option<u64> {
        session_volume(self)
    }
}

impl Bar {
    #[must_use]
    pub fn point(&self, daily: bool) -> BarPoint {
        let raw_timestamp = self.t.clone().unwrap_or_default();
        let timestamp = if daily {
            raw_timestamp
                .get(..10)
                .unwrap_or(raw_timestamp.as_str())
                .to_owned()
        } else {
            raw_timestamp
        };

        BarPoint {
            timestamp,
            open: self.o.unwrap_or_default(),
            high: self.h.unwrap_or_default(),
            low: self.l.unwrap_or_default(),
            close: self.c.unwrap_or_default(),
            volume: match self.v {
                Some(value) => i64::try_from(value).unwrap_or(i64::MAX),
                None => 0,
            },
        }
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
pub fn preferred_feed(extended_hours: bool) -> DataFeed {
    if extended_hours {
        DataFeed::Boats
    } else {
        DataFeed::Sip
    }
}
