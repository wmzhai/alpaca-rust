use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub date: String,
    pub open: String,
    pub close: String,
    pub session_open: String,
    pub session_close: String,
    pub settlement_date: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarV3Response {
    pub market: CalendarMarket,
    pub calendar: Vec<CalendarDay>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarMarket {
    pub mic: Option<String>,
    pub bic: Option<String>,
    pub acronym: String,
    pub name: String,
    pub timezone: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDay {
    pub date: String,
    pub pre_start: Option<String>,
    pub pre_end: Option<String>,
    pub lunch_start: Option<String>,
    pub lunch_end: Option<String>,
    pub core_start: String,
    pub core_end: String,
    pub post_start: Option<String>,
    pub post_end: Option<String>,
    pub settlement_date: Option<String>,
}
