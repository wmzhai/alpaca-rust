use alpaca_trade::options_contracts::{
    ContractStatus, ContractStyle, ContractType, ListResponse, OptionContract,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::auth::MockHttpError;
use crate::state::{ListOptionContractsFilter, MockServerState};

pub(crate) async fn options_contracts_get(
    State(state): State<MockServerState>,
    Path(symbol_or_id): Path<String>,
) -> Result<Json<OptionContract>, MockHttpError> {
    state
        .get_option_contract(&symbol_or_id)
        .map(Json)
        .ok_or_else(|| MockHttpError::not_found("option contract was not found"))
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ListOptionContractsQuery {
    underlying_symbols: Option<String>,
    show_deliverables: Option<bool>,
    status: Option<ContractStatus>,
    expiration_date: Option<String>,
    expiration_date_gte: Option<String>,
    expiration_date_lte: Option<String>,
    root_symbol: Option<String>,
    #[serde(rename = "type")]
    contract_type: Option<ContractType>,
    style: Option<ContractStyle>,
    strike_price_gte: Option<Decimal>,
    strike_price_lte: Option<Decimal>,
    page_token: Option<String>,
    limit: Option<u32>,
    ppind: Option<bool>,
}

pub(crate) async fn options_contracts_list(
    State(state): State<MockServerState>,
    Query(query): Query<ListOptionContractsQuery>,
) -> Result<Json<ListResponse>, MockHttpError> {
    if !matches!(query.limit, None | Some(1..=10_000)) {
        return Err(MockHttpError::bad_request(
            "limit must be between 1 and 10000",
        ));
    }
    if query
        .strike_price_gte
        .zip(query.strike_price_lte)
        .is_some_and(|(gte, lte)| gte > lte)
    {
        return Err(MockHttpError::bad_request(
            "strike_price_gte must not exceed strike_price_lte",
        ));
    }

    Ok(Json(state.list_option_contracts(
        ListOptionContractsFilter {
            underlying_symbols: parse_csv(query.underlying_symbols)?,
            show_deliverables: query.show_deliverables,
            status: query.status,
            expiration_date: query.expiration_date,
            expiration_date_gte: query.expiration_date_gte,
            expiration_date_lte: query.expiration_date_lte,
            root_symbol: query.root_symbol,
            contract_type: query.contract_type,
            style: query.style,
            strike_price_gte: query.strike_price_gte,
            strike_price_lte: query.strike_price_lte,
            page_token: query.page_token,
            limit: query.limit,
            ppind: query.ppind,
        },
    )))
}

fn parse_csv(value: Option<String>) -> Result<Option<Vec<String>>, MockHttpError> {
    value
        .map(|value| {
            let values = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if values.is_empty() {
                Err(MockHttpError::bad_request(
                    "underlying_symbols must not be empty",
                ))
            } else {
                Ok(values)
            }
        })
        .transpose()
}
