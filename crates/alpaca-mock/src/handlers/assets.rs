use alpaca_trade::assets::{Asset, AssetAttribute, AssetClass, AssetStatus, Exchange};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::auth::MockHttpError;
use crate::state::{ListAssetsFilter, MockServerState};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ListAssetsQuery {
    status: Option<AssetStatus>,
    asset_class: Option<AssetClass>,
    exchange: Option<Exchange>,
    attributes: Option<String>,
}

pub(crate) async fn assets_get(
    State(state): State<MockServerState>,
    Path(symbol_or_asset_id): Path<String>,
) -> Result<Json<Asset>, MockHttpError> {
    state
        .get_asset(&symbol_or_asset_id)
        .map(Json)
        .ok_or_else(|| MockHttpError::not_found("asset was not found"))
}

pub(crate) async fn assets_list(
    State(state): State<MockServerState>,
    Query(query): Query<ListAssetsQuery>,
) -> Result<Json<Vec<Asset>>, MockHttpError> {
    Ok(Json(state.list_assets(ListAssetsFilter {
        status: query.status,
        asset_class: query.asset_class,
        exchange: query.exchange,
        attributes: parse_attributes(query.attributes)?,
    })))
}

fn parse_attributes(value: Option<String>) -> Result<Option<Vec<AssetAttribute>>, MockHttpError> {
    value
        .map(|value| {
            value
                .split(',')
                .map(|value| match value.trim() {
                    "ptp_no_exception" => Ok(AssetAttribute::PtpNoException),
                    "ptp_with_exception" => Ok(AssetAttribute::PtpWithException),
                    "ipo" => Ok(AssetAttribute::Ipo),
                    "has_options" => Ok(AssetAttribute::HasOptions),
                    "options_late_close" => Ok(AssetAttribute::OptionsLateClose),
                    "fractional_eh_enabled" => Ok(AssetAttribute::FractionalEhEnabled),
                    "overnight_tradable" => Ok(AssetAttribute::OvernightTradable),
                    "overnight_halted" => Ok(AssetAttribute::OvernightHalted),
                    _ => Err(MockHttpError::bad_request("unsupported asset attribute")),
                })
                .collect()
        })
        .transpose()
}
