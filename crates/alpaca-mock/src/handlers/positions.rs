use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde::Deserialize;

use alpaca_trade::positions::{
    ClosePositionBody, ClosePositionResult, ExercisePositionBody, Position,
};

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::{ClosePositionInput, MockServerState};

#[derive(Debug, Deserialize, Default)]
pub(crate) struct CloseAllPositionsQuery {
    cancel_orders: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct ClosePositionQuery {
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    qty: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    percentage: Option<Decimal>,
}

pub(crate) async fn positions_list(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
) -> Result<Json<Vec<Position>>, MockHttpError> {
    Ok(Json(state.list_positions(&account.api_key).await?))
}

pub(crate) async fn positions_get(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(symbol_or_asset_id): Path<String>,
) -> Result<Json<Position>, MockHttpError> {
    Ok(Json(
        state
            .get_position(&account.api_key, &symbol_or_asset_id)
            .await?,
    ))
}

pub(crate) async fn positions_close(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(symbol_or_asset_id): Path<String>,
    Query(query): Query<ClosePositionQuery>,
) -> Result<Json<ClosePositionBody>, MockHttpError> {
    Ok(Json(
        state
            .close_position(
                &account.api_key,
                &symbol_or_asset_id,
                ClosePositionInput {
                    qty: query.qty,
                    percentage: query.percentage,
                },
            )
            .await?,
    ))
}

pub(crate) async fn positions_close_all(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<CloseAllPositionsQuery>,
) -> Result<(StatusCode, Json<Vec<ClosePositionResult>>), MockHttpError> {
    Ok((
        StatusCode::MULTI_STATUS,
        Json(
            state
                .close_all_positions(&account.api_key, query.cancel_orders.unwrap_or(false))
                .await?,
        ),
    ))
}

pub(crate) async fn positions_exercise(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(symbol_or_contract_id): Path<String>,
) -> Result<Json<ExercisePositionBody>, MockHttpError> {
    Ok(Json(state.exercise_position(
        &account.api_key,
        &symbol_or_contract_id,
    )?))
}

pub(crate) async fn positions_do_not_exercise(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(symbol_or_contract_id): Path<String>,
) -> Result<StatusCode, MockHttpError> {
    state.do_not_exercise_position(&account.api_key, &symbol_or_contract_id)?;
    Ok(StatusCode::NO_CONTENT)
}
