use alpaca_trade::{
    calendar::Market,
    clock::{Clock, ClockV3Response},
};
use axum::{
    Json,
    extract::{Query, State},
};
use chrono::DateTime;
use serde::Deserialize;

use crate::auth::MockHttpError;
use crate::state::MockServerState;

pub(crate) async fn clock_legacy(State(state): State<MockServerState>) -> Json<Clock> {
    Json(state.legacy_clock())
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ClockV3Query {
    markets: Option<String>,
    time: Option<String>,
}

pub(crate) async fn clock_v3(
    State(state): State<MockServerState>,
    Query(query): Query<ClockV3Query>,
) -> Result<Json<ClockV3Response>, MockHttpError> {
    let markets = query
        .markets
        .map(|markets| {
            markets
                .split(',')
                .map(|market| {
                    serde_json::from_value::<Market>(serde_json::Value::String(market.to_owned()))
                        .map_err(|_| MockHttpError::bad_request("unsupported market"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_else(|| vec![Market::NYSE]);
    if markets.is_empty() {
        return Err(MockHttpError::bad_request(
            "markets must contain at least one value",
        ));
    }

    let time = query
        .time
        .map(|time| {
            DateTime::parse_from_rfc3339(&time)
                .map(|_| time)
                .map_err(|_| MockHttpError::bad_request("time must use RFC3339 format"))
        })
        .transpose()?;
    Ok(Json(state.clock_v3(markets, time)))
}
