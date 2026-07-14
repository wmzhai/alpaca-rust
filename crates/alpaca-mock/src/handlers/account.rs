use alpaca_trade::{
    account::Account,
    account_configurations::{AccountConfigurations, UpdateRequest},
};
use axum::{Json, extract::State};

use crate::auth::AuthenticatedAccount;
use crate::state::MockServerState;

pub(crate) async fn account_get(
    State(state): State<MockServerState>,
    axum::extract::Extension(account): axum::extract::Extension<AuthenticatedAccount>,
) -> Json<Account> {
    Json(state.project_account(&account.api_key))
}

pub(crate) async fn account_configurations_get(
    State(state): State<MockServerState>,
    axum::extract::Extension(account): axum::extract::Extension<AuthenticatedAccount>,
) -> Json<AccountConfigurations> {
    Json(state.project_account_configurations(&account.api_key))
}

pub(crate) async fn account_configurations_update(
    State(state): State<MockServerState>,
    axum::extract::Extension(account): axum::extract::Extension<AuthenticatedAccount>,
    Json(request): Json<UpdateRequest>,
) -> Json<AccountConfigurations> {
    Json(state.update_account_configurations(&account.api_key, request))
}
