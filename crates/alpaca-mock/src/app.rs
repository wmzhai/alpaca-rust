use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderValue, Request, header::HeaderName},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::auth::{AuthenticatedAccount, MockHttpError, extract_auth};
use crate::handlers;
use crate::state::{MarketDataBridgeError, MockServerState};

static MOCK_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub fn build_app() -> Router {
    build_app_with_state(MockServerState::new())
}

pub fn build_app_from_env() -> Result<Router, MarketDataBridgeError> {
    Ok(build_app_with_state(MockServerState::from_env()?))
}

pub fn build_app_with_state(state: MockServerState) -> Router {
    let trading_router = Router::new()
        .route("/v2/account", get(handlers::account_get))
        .route(
            "/v2/account/configurations",
            get(handlers::account_configurations_get)
                .patch(handlers::account_configurations_update),
        )
        .route(
            "/v2/account/portfolio/history",
            get(handlers::portfolio_history_get),
        )
        .route("/v2/assets", get(handlers::assets_list))
        .route("/v2/assets/{symbol_or_asset_id}", get(handlers::assets_get))
        .route(
            "/v2/options/contracts",
            get(handlers::options_contracts_list),
        )
        .route(
            "/v2/options/contracts/{symbol_or_id}",
            get(handlers::options_contracts_get),
        )
        .route("/v2/calendar", get(handlers::calendar_legacy))
        .route("/v3/calendar/{market}", get(handlers::calendar_v3))
        .route("/v2/clock", get(handlers::clock_legacy))
        .route("/v3/clock", get(handlers::clock_v3))
        .route("/v2/account/activities", get(handlers::activities_list))
        .route(
            "/v2/account/activities/{activity_type}",
            get(handlers::activities_by_type),
        )
        .route(
            "/v2/orders",
            get(handlers::orders_list)
                .post(handlers::orders_create)
                .delete(handlers::orders_cancel_all),
        )
        .route(
            "/v2/orders/{order_id}",
            get(handlers::orders_get)
                .patch(handlers::orders_replace)
                .delete(handlers::orders_cancel),
        )
        .route(
            "/v2/orders:by_client_order_id",
            get(handlers::orders_get_by_client_order_id),
        )
        .route(
            "/v2/watchlists",
            get(handlers::watchlists_list).post(handlers::watchlists_create),
        )
        .route(
            "/v2/watchlists:by_name",
            get(handlers::watchlists_get_by_name)
                .put(handlers::watchlists_update_by_name)
                .post(handlers::watchlists_add_asset_by_name)
                .delete(handlers::watchlists_delete_by_name),
        )
        .route(
            "/v2/watchlists/{watchlist_id}",
            get(handlers::watchlists_get_by_id)
                .put(handlers::watchlists_update_by_id)
                .post(handlers::watchlists_add_asset_by_id)
                .delete(handlers::watchlists_delete_by_id),
        )
        .route(
            "/v2/watchlists/{watchlist_id}/{symbol}",
            axum::routing::delete(handlers::watchlists_remove_asset_by_id),
        )
        .route(
            "/v2/positions",
            get(handlers::positions_list).delete(handlers::positions_close_all),
        )
        .route(
            "/v2/positions/{symbol_or_asset_id}",
            get(handlers::positions_get).delete(handlers::positions_close),
        )
        .route(
            "/v2/positions/{symbol_or_contract_id}/exercise",
            post(handlers::positions_exercise),
        )
        .route(
            "/v2/positions/{symbol_or_contract_id}/do-not-exercise",
            post(handlers::positions_do_not_exercise),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_trading_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health))
        .route("/reset", post(handlers::admin_reset))
        .route("/admin/state", get(handlers::admin_state))
        .route("/admin/reset", post(handlers::admin_reset))
        .route("/admin/faults/http", post(handlers::admin_set_http_fault))
        .route(
            "/admin/fixtures/rejected-replacement-race",
            post(handlers::admin_seed_rejected_replacement_race),
        )
        .merge(trading_router)
        .with_state(state)
}

async fn require_trading_auth(
    State(state): State<MockServerState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, MockHttpError> {
    let auth = extract_auth(request.headers())?;
    let _secret_key = auth.secret_key;
    state.ensure_account(&auth.api_key);

    if let Some(fault) = state.take_http_fault() {
        let status = fault.status_code().map_err(MockHttpError::bad_request)?;
        return Err(MockHttpError::with_status(status, fault.message));
    }

    request.extensions_mut().insert(AuthenticatedAccount {
        api_key: auth.api_key,
    });

    let mut response = next.run(request).await;
    let request_id = MOCK_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    response.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&format!("mock-{request_id:016x}"))
            .expect("generated mock request id should be a valid header value"),
    );

    Ok(response)
}
