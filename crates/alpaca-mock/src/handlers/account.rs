use alpaca_trade::account::Account;
use axum::{Json, extract::State};

use crate::auth::AuthenticatedAccount;
use crate::state::MockServerState;

pub(crate) async fn account_get(
    State(state): State<MockServerState>,
    axum::extract::Extension(account): axum::extract::Extension<AuthenticatedAccount>,
) -> Json<Account> {
    Json(state.project_account(&account.api_key))
}
