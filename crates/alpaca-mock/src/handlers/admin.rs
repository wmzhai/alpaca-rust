use axum::{Json, extract::State};
use serde::Deserialize;

use crate::auth::MockHttpError;
use crate::state::{AdminStateResponse, InjectedHttpFault, MockServerState};

#[derive(Debug, Deserialize)]
pub struct InjectHttpFaultRequest {
    pub status: u16,
    pub message: String,
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
