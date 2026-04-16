use std::collections::HashMap;

use rust_decimal::Decimal;

use super::Snapshot;

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;

    use super::{Snapshot, ordered_snapshots};
    use crate::stocks::{Bar, Quote, Trade};

    #[test]
    fn snapshot_timestamp_prefers_the_freshest_available_value() {
        let snapshot = Snapshot {
            latest_trade: Some(Trade {
                t: Some("2026-04-13T13:30:01Z".to_owned()),
                ..Trade::default()
            }),
            latest_quote: Some(Quote {
                t: Some("2026-04-13T13:30:05Z".to_owned()),
                ..Quote::default()
            }),
            minute_bar: Some(Bar {
                t: Some("2026-04-13T13:30:00Z".to_owned()),
                ..Bar::default()
            }),
            ..Snapshot::default()
        };

        assert_eq!(snapshot.timestamp(), Some("2026-04-13T13:30:05Z"));
    }

    #[test]
    fn snapshot_price_absorbs_single_sided_quotes_and_trade_fallbacks() {
        let with_both_sides = Snapshot {
            latest_quote: Some(Quote {
                bp: Some(Decimal::new(125, 2)),
                ap: Some(Decimal::new(135, 2)),
                ..Quote::default()
            }),
            ..Snapshot::default()
        };
        let with_bid_only = Snapshot {
            latest_quote: Some(Quote {
                bp: Some(Decimal::new(125, 2)),
                ..Quote::default()
            }),
            ..Snapshot::default()
        };
        let with_trade = Snapshot {
            latest_trade: Some(Trade {
                p: Some(Decimal::new(141, 2)),
                ..Trade::default()
            }),
            latest_quote: Some(Quote {
                bp: Some(Decimal::new(125, 2)),
                ap: Some(Decimal::new(135, 2)),
                ..Quote::default()
            }),
            ..Snapshot::default()
        };

        assert_eq!(with_both_sides.price(), Some(Decimal::new(130, 2)));
        assert_eq!(with_bid_only.price(), Some(Decimal::new(125, 2)));
        assert_eq!(with_trade.price(), Some(Decimal::new(141, 2)));
    }

    #[test]
    fn snapshot_canonical_session_readers_hide_provider_nesting() {
        let snapshot = Snapshot {
            latest_quote: Some(Quote {
                bp: Some(Decimal::new(50000, 2)),
                ap: Some(Decimal::new(50030, 2)),
                ..Quote::default()
            }),
            daily_bar: Some(Bar {
                o: Some(Decimal::new(49810, 2)),
                h: Some(Decimal::new(50320, 2)),
                l: Some(Decimal::new(49750, 2)),
                c: Some(Decimal::new(50140, 2)),
                v: Some(1_234_567),
                ..Bar::default()
            }),
            prev_daily_bar: Some(Bar {
                c: Some(Decimal::new(49680, 2)),
                ..Bar::default()
            }),
            ..Snapshot::default()
        };

        assert_eq!(snapshot.bid_price(), Some(Decimal::new(50000, 2)));
        assert_eq!(snapshot.ask_price(), Some(Decimal::new(50030, 2)));
        assert_eq!(snapshot.session_open(), Some(Decimal::new(49810, 2)));
        assert_eq!(snapshot.session_high(), Some(Decimal::new(50320, 2)));
        assert_eq!(snapshot.session_low(), Some(Decimal::new(49750, 2)));
        assert_eq!(snapshot.session_close(), Some(Decimal::new(50140, 2)));
        assert_eq!(snapshot.previous_close(), Some(Decimal::new(49680, 2)));
        assert_eq!(snapshot.session_volume(), Some(1_234_567));
    }

    #[test]
    fn ordered_snapshots_returns_stable_symbol_order() {
        let mut snapshots = HashMap::new();
        snapshots.insert("QQQ".to_owned(), Snapshot::default());
        snapshots.insert("AAPL".to_owned(), Snapshot::default());

        let ordered = ordered_snapshots(&snapshots);
        assert_eq!(ordered[0].0, "AAPL");
        assert_eq!(ordered[1].0, "QQQ");
    }
}
