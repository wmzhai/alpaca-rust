use alpaca_trade::calendar::{Calendar, CalendarTimezone, CalendarV3Response, DateType, Market};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::auth::MockHttpError;
use crate::state::MockServerState;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LegacyCalendarQuery {
    start: Option<String>,
    end: Option<String>,
    date_type: Option<DateType>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CalendarV3Query {
    start: Option<String>,
    end: Option<String>,
    timezone: Option<CalendarTimezone>,
}

pub(crate) async fn calendar_legacy(
    State(state): State<MockServerState>,
    Query(query): Query<LegacyCalendarQuery>,
) -> Result<Json<Vec<Calendar>>, MockHttpError> {
    let start = parse_date("start", query.start)?;
    let end = parse_date("end", query.end)?;
    if start.zip(end).is_some_and(|(start, end)| start > end) {
        return Err(MockHttpError::bad_request("start must not be after end"));
    }

    Ok(Json(state.legacy_calendar(
        start,
        end,
        query.date_type.unwrap_or(DateType::Trading),
    )))
}

pub(crate) async fn calendar_v3(
    State(state): State<MockServerState>,
    Path(market): Path<Market>,
    Query(query): Query<CalendarV3Query>,
) -> Result<Json<CalendarV3Response>, MockHttpError> {
    let start = parse_date("start", query.start)?;
    let end = parse_date("end", query.end)?;
    if start.zip(end).is_some_and(|(start, end)| start > end) {
        return Err(MockHttpError::bad_request("start must not be after end"));
    }

    Ok(Json(state.calendar_v3(market, start, end, query.timezone)))
}

fn parse_date(name: &str, value: Option<String>) -> Result<Option<NaiveDate>, MockHttpError> {
    value
        .map(|value| {
            NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
                MockHttpError::bad_request(format!("{name} must use YYYY-MM-DD format"))
            })
        })
        .transpose()
}
