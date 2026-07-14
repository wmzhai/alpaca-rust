use alpaca_trade::portfolio_history::PortfolioHistory;
use axum::{
    Json,
    extract::{Extension, Query, State},
};
use serde::Deserialize;

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::MockServerState;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct PortfolioHistoryQuery {
    period: Option<String>,
    timeframe: Option<String>,
    intraday_reporting: Option<String>,
    start: Option<String>,
    pnl_reset: Option<String>,
    end: Option<String>,
    extended_hours: Option<String>,
    cashflow_types: Option<String>,
}

pub(crate) async fn portfolio_history_get(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<PortfolioHistoryQuery>,
) -> Result<Json<PortfolioHistory>, MockHttpError> {
    query.validate()?;
    Ok(Json(state.project_portfolio_history(
        &account.api_key,
        query.timeframe.as_deref().unwrap_or("1D"),
    )))
}

impl PortfolioHistoryQuery {
    fn validate(&self) -> Result<(), MockHttpError> {
        let range_fields = [
            self.period.is_some(),
            self.start.is_some(),
            self.end.is_some(),
        ]
        .into_iter()
        .filter(|is_some| *is_some)
        .count();
        if range_fields > 2 {
            return Err(MockHttpError::bad_request(
                "only two of period, start, and end may be specified",
            ));
        }
        if self
            .timeframe
            .as_deref()
            .is_some_and(|value| !matches!(value, "1Min" | "5Min" | "15Min" | "1H" | "1D"))
        {
            return Err(MockHttpError::bad_request("unsupported timeframe"));
        }
        if self
            .intraday_reporting
            .as_deref()
            .is_some_and(|value| !matches!(value, "market_hours" | "extended_hours" | "continuous"))
        {
            return Err(MockHttpError::bad_request("unsupported intraday_reporting"));
        }
        if self
            .pnl_reset
            .as_deref()
            .is_some_and(|value| !matches!(value, "no_reset" | "per_day"))
        {
            return Err(MockHttpError::bad_request("unsupported pnl_reset"));
        }
        for (name, value) in [
            ("period", self.period.as_deref()),
            ("start", self.start.as_deref()),
            ("end", self.end.as_deref()),
            ("extended_hours", self.extended_hours.as_deref()),
            ("cashflow_types", self.cashflow_types.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(MockHttpError::bad_request(format!(
                    "{name} must not be empty"
                )));
            }
        }
        Ok(())
    }
}
