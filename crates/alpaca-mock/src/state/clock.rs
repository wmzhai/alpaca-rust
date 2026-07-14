use alpaca_trade::{
    calendar::Market,
    clock::{Clock, ClockMarket, ClockV3, ClockV3Response, MarketPhase},
};
use chrono::{SecondsFormat, Utc};

pub(crate) fn legacy_clock() -> Clock {
    Clock {
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        is_open: false,
        next_open: "2026-07-14T09:30:00-04:00".to_owned(),
        next_close: "2026-07-14T16:00:00-04:00".to_owned(),
    }
}

pub(crate) fn clock_v3(markets: Vec<Market>, time: Option<String>) -> ClockV3Response {
    ClockV3Response {
        clocks: markets
            .into_iter()
            .map(|market| ClockV3 {
                market: ClockMarket {
                    mic: (market == Market::NYSE).then(|| "XNYS".to_owned()),
                    bic: None,
                    acronym: market.to_string(),
                    name: if market == Market::NYSE {
                        "New York Stock Exchange".to_owned()
                    } else {
                        market.to_string()
                    },
                    timezone: if market == Market::NYSE {
                        "America/New_York".to_owned()
                    } else {
                        "UTC".to_owned()
                    },
                },
                timestamp: if market == Market::NYSE
                    && time.as_deref() == Some("2026-07-13T15:00:00Z")
                {
                    "2026-07-13T11:00:00-04:00".to_owned()
                } else {
                    time.clone()
                        .unwrap_or_else(|| Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true))
                },
                is_market_day: true,
                next_market_open: "2026-07-14T09:30:00-04:00".to_owned(),
                next_market_close: "2026-07-13T16:00:00-04:00".to_owned(),
                phase: if time.as_deref() == Some("2026-07-13T15:00:00Z") {
                    MarketPhase::Core
                } else {
                    MarketPhase::Closed
                },
                phase_until: "2026-07-13T16:00:00-04:00".to_owned(),
            })
            .collect(),
    }
}
