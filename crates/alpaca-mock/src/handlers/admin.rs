use axum::{
    Json,
    extract::{Path, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;

use alpaca_trade::orders::OrderStatus;

use crate::auth::MockHttpError;
use crate::state::{
    AdminStateResponse, InjectedHttpFault, MockServerState, RejectedReplacementRaceFixture,
    RuntimeStockPriceResponse,
};

#[derive(Debug, Deserialize)]
pub struct InjectHttpFaultRequest {
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SeedRejectedReplacementRaceRequest {
    pub api_key: String,
    #[serde(default = "filled_predecessor_status")]
    pub predecessor_status: OrderStatus,
}

fn filled_predecessor_status() -> OrderStatus {
    OrderStatus::Filled
}

#[derive(Debug, Deserialize)]
pub struct SetRuntimeStockPriceRequest {
    #[serde(deserialize_with = "alpaca_core::decimal::price_string_contract::deserialize")]
    pub price: Decimal,
}

pub(crate) async fn admin_state(State(state): State<MockServerState>) -> Json<AdminStateResponse> {
    Json(state.admin_state())
}

pub(crate) async fn admin_reset(State(state): State<MockServerState>) -> Json<AdminStateResponse> {
    state.reset();
    Json(state.admin_state())
}

pub(crate) async fn admin_set_http_fault(
    State(state): State<MockServerState>,
    Json(request): Json<InjectHttpFaultRequest>,
) -> Result<Json<AdminStateResponse>, MockHttpError> {
    let fault = InjectedHttpFault::new(request.status, request.message)
        .map_err(MockHttpError::bad_request)?;
    state.set_http_fault(fault);
    Ok(Json(state.admin_state()))
}

pub(crate) async fn admin_set_runtime_stock_price(
    State(state): State<MockServerState>,
    Path(symbol): Path<String>,
    Json(request): Json<SetRuntimeStockPriceRequest>,
) -> Result<Json<RuntimeStockPriceResponse>, MockHttpError> {
    state
        .set_runtime_stock_price(&symbol, request.price)
        .map(Json)
        .map_err(|error| MockHttpError::bad_request(error.to_string()))
}

pub(crate) async fn admin_seed_rejected_replacement_race(
    State(state): State<MockServerState>,
    Json(request): Json<SeedRejectedReplacementRaceRequest>,
) -> Json<RejectedReplacementRaceFixture> {
    Json(state.seed_rejected_replacement_race(&request.api_key, request.predecessor_status))
}
