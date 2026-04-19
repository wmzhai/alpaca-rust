use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayCountBasis {
    #[serde(rename = "ACT/365")]
    Act365,
    #[serde(rename = "ACT/365.25")]
    Act36525,
    #[serde(rename = "ACT/360")]
    Act360,
}

impl DayCountBasis {
    pub fn from_option_str(value: Option<&str>) -> Self {
        match value.unwrap_or("ACT/365.25") {
            "ACT/365" => Self::Act365,
            "ACT/360" => Self::Act360,
            _ => Self::Act36525,
        }
    }

    pub fn denominator(self) -> f64 {
        match self {
            Self::Act365 => 365.0,
            Self::Act36525 => 365.25,
            Self::Act360 => 360.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketSession {
    Premarket,
    Regular,
    AfterHours,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekdayCode {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketHours {
    pub date: String,
    pub is_trading_date: bool,
    pub is_early_close: bool,
    pub premarket_open: Option<String>,
    pub regular_open: Option<String>,
    pub regular_close: Option<String>,
    pub after_hours_close: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradingDayInfo {
    pub date: String,
    pub is_trading_date: bool,
    pub is_market_holiday: bool,
    pub is_early_close: bool,
    pub market_hours: MarketHours,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationParts {
    pub sign: i8,
    pub total_seconds: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampParts {
    pub date: String,
    pub timestamp: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub hhmm: u32,
    pub hhmm_string: String,
    pub weekday_from_sunday: u32,
}
