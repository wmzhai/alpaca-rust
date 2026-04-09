use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub timestamp: String,
    pub is_open: bool,
    pub next_open: String,
    pub next_close: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockV3Response {
    pub clocks: Vec<ClockV3>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockV3 {
    pub market: ClockMarket,
    pub timestamp: String,
    pub is_market_day: bool,
    pub next_market_open: String,
    pub next_market_close: String,
    pub phase: String,
    pub phase_until: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockMarket {
    pub mic: Option<String>,
    pub bic: Option<String>,
    pub acronym: String,
    pub name: String,
    pub timezone: String,
}
