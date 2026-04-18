use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{OptionsFeed, Snapshot};

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
pub fn preferred_feed() -> OptionsFeed {
    OptionsFeed::Opra
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;

    use super::{Snapshot, ordered_snapshots, preferred_feed};
    use crate::options::{Bar, OptionsFeed, Quote, Trade};

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
    fn snapshot_mark_price_absorbs_single_sided_quotes() {
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

        assert_eq!(with_both_sides.mark_price(), Some(Decimal::new(130, 2)));
        assert_eq!(with_bid_only.mark_price(), Some(Decimal::new(125, 2)));
    }

    #[test]
    fn ordered_snapshots_returns_stable_contract_order() {
        let mut snapshots = HashMap::new();
        snapshots.insert("QQQ250620C00500000".to_owned(), Snapshot::default());
        snapshots.insert("AAPL250620C00200000".to_owned(), Snapshot::default());

        let ordered = ordered_snapshots(&snapshots);
        assert_eq!(ordered[0].0, "AAPL250620C00200000");
        assert_eq!(ordered[1].0, "QQQ250620C00500000");
    }

    #[test]
    fn preferred_feed_uses_opra() {
        assert_eq!(preferred_feed(), OptionsFeed::Opra);
    }
}
