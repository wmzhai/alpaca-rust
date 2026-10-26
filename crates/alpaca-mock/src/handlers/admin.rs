use axum::{
    Json,
    extract::{Path, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;

use alpaca_trade::orders::{OrderSide, OrderStatus};

use crate::auth::MockHttpError;
use crate::state::{
    AdminStateResponse, ImportOrdersResponse, ImportedLimitOrder, InjectedHttpFault,
    MockServerState, RejectedReplacementRaceFixture, RuntimeStockPriceResponse,
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

#[derive(Debug, Deserialize)]
pub struct ImportOrdersRequest {
    pub api_key: String,
    pub orders: Vec<ImportOrderRequest>,
}

#[derive(Debug, Deserialize)]
pub struct ImportOrderRequest {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    #[serde(deserialize_with = "alpaca_core::decimal::number_contract::deserialize")]
    pub qty: Decimal,
    #[serde(deserialize_with = "alpaca_core::decimal::price_string_contract::deserialize")]
    pub limit_price: Decimal,
}

pub(crate) async fn admin_import_orders(
    State(state): State<MockServerState>,
    Json(request): Json<ImportOrdersRequest>,
) -> Result<Json<ImportOrdersResponse>, MockHttpError> {
    if request.api_key.is_empty() {
        return Err(MockHttpError::bad_request("api_key must not be empty"));
    }
    let orders = request
        .orders
        .into_iter()
        .map(|order| ImportedLimitOrder {
            id: order.id,
            client_order_id: order.client_order_id,
            symbol: order.symbol,
            side: order.side,
            qty: order.qty,
            limit_price: order.limit_price,
        })
        .collect();
    state
        .import_orders(&request.api_key, orders)
        .map(Json)
        .map_err(|error| MockHttpError::bad_request(error.to_string()))
}
