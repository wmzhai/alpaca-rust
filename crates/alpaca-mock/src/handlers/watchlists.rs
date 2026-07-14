use alpaca_trade::watchlists::{
    AddAssetRequest, CreateRequest, UpdateRequest, Watchlist, WatchlistSummary,
};
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::{MockServerState, MockStateError};

type RouteResult<T> = Result<T, MockHttpError>;

#[derive(Debug, Deserialize)]
pub(crate) struct WatchlistNameQuery {
    name: String,
}

pub(crate) async fn watchlists_list(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
) -> Json<Vec<WatchlistSummary>> {
    Json(state.list_watchlists(&account.api_key))
}

pub(crate) async fn watchlists_create(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Json(request): Json<CreateRequest>,
) -> RouteResult<Json<Watchlist>> {
    request.validate().map_err(invalid_request)?;
    state
        .create_watchlist(&account.api_key, request.name, request.symbols)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_get_by_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(watchlist_id): Path<String>,
) -> RouteResult<Json<Watchlist>> {
    state
        .get_watchlist_by_id(&account.api_key, &watchlist_id)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_update_by_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(watchlist_id): Path<String>,
    Json(request): Json<UpdateRequest>,
) -> RouteResult<Json<Watchlist>> {
    request.validate().map_err(invalid_request)?;
    state
        .update_watchlist_by_id(
            &account.api_key,
            &watchlist_id,
            request.name,
            request.symbols,
        )
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_delete_by_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(watchlist_id): Path<String>,
) -> RouteResult<StatusCode> {
    state
        .delete_watchlist_by_id(&account.api_key, &watchlist_id)
        .map_err(state_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn watchlists_add_asset_by_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(watchlist_id): Path<String>,
    Json(request): Json<AddAssetRequest>,
) -> RouteResult<Json<Watchlist>> {
    request.validate().map_err(invalid_request)?;
    state
        .add_watchlist_asset_by_id(&account.api_key, &watchlist_id, &request.symbol)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_remove_asset_by_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path((watchlist_id, symbol)): Path<(String, String)>,
) -> RouteResult<Json<Watchlist>> {
    state
        .remove_watchlist_asset_by_id(&account.api_key, &watchlist_id, &symbol)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_get_by_name(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<WatchlistNameQuery>,
) -> RouteResult<Json<Watchlist>> {
    validate_name(&query.name)?;
    state
        .get_watchlist_by_name(&account.api_key, &query.name)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_update_by_name(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<WatchlistNameQuery>,
    Json(request): Json<UpdateRequest>,
) -> RouteResult<Json<Watchlist>> {
    validate_name(&query.name)?;
    request.validate().map_err(invalid_request)?;
    state
        .update_watchlist_by_name(&account.api_key, &query.name, request.name, request.symbols)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_add_asset_by_name(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<WatchlistNameQuery>,
    Json(request): Json<AddAssetRequest>,
) -> RouteResult<Json<Watchlist>> {
    validate_name(&query.name)?;
    request.validate().map_err(invalid_request)?;
    state
        .add_watchlist_asset_by_name(&account.api_key, &query.name, &request.symbol)
        .map(Json)
        .map_err(state_error)
}

pub(crate) async fn watchlists_delete_by_name(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<WatchlistNameQuery>,
) -> RouteResult<StatusCode> {
    validate_name(&query.name)?;
    state
        .delete_watchlist_by_name(&account.api_key, &query.name)
        .map_err(state_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_name(name: &str) -> RouteResult<()> {
    UpdateRequest {
        name: Some(name.to_owned()),
        symbols: None,
    }
    .validate()
    .map_err(invalid_request)
}

fn invalid_request(error: alpaca_trade::Error) -> MockHttpError {
    MockHttpError::conflict(error.to_string())
}

fn state_error(error: MockStateError) -> MockHttpError {
    match error {
        MockStateError::NotFound(message) => MockHttpError::not_found(message),
        MockStateError::Forbidden(message) => {
            MockHttpError::with_status(StatusCode::FORBIDDEN, message)
        }
        MockStateError::Conflict(message) => MockHttpError::conflict(message),
        MockStateError::MarketDataUnavailable(message) => MockHttpError::internal(message),
    }
}
