use axum::{
    Router,
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};

use crate::auth::{AuthenticatedAccount, MockHttpError, extract_auth};
use crate::handlers;
use crate::state::{MarketDataBridgeError, MockServerState};

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
        .route("/admin/state", get(handlers::admin_state))
        .route("/admin/reset", post(handlers::admin_reset))
        .route("/admin/faults/http", post(handlers::admin_set_http_fault))
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

    if let Some(fault) = state.http_fault() {
        let status = fault.status_code().map_err(MockHttpError::bad_request)?;
        return Err(MockHttpError::with_status(status, fault.message));
    }

    request.extensions_mut().insert(AuthenticatedAccount {
        api_key: auth.api_key,
    });

    Ok(next.run(request).await)
}
