use std::fmt;
use std::sync::Arc;

use alpaca_http::{NoContent, RequestParts};
use reqwest::Method;
use serde::Deserialize;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;

use crate::client::ClientInner;
use crate::positions::{
    reconcile_signed_positions, structure_quantity, CloseAllRequest, ClosePositionBody,
    ClosePositionRequest, ClosePositionResult, DoNotExerciseAccepted, ExercisePositionBody,
    Position,
};
use crate::{Error, positions::request};

#[derive(Clone)]
pub struct PositionsClient {
    inner: Arc<ClientInner>,
}

impl PositionsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self) -> Result<Vec<Position>, Error> {
        let request =
            RequestParts::new(Method::GET, "/v2/positions").with_operation("positions.list");

        self.inner
            .send_json::<Vec<Position>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn option_qty_map(&self) -> Result<HashMap<String, i32>, Error> {
        #[derive(Debug, Deserialize)]
        struct PositionQtyRow {
            symbol: String,
            #[serde(deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number")]
            qty: Decimal,
        }

        let request =
            RequestParts::new(Method::GET, "/v2/positions").with_operation("positions.option_qty_map");

        let rows = self
            .inner
            .send_json::<Vec<PositionQtyRow>>(request)
            .await
            .map(|response| response.into_body())?;

        let mut mapped = HashMap::new();
        for row in rows {
            let contract = row.symbol.trim();
            if contract.len() <= 10 {
                continue;
            }

            mapped.insert(
                contract.to_string(),
                row.qty.trunc().to_i32().unwrap_or(0),
            );
        }

        Ok(mapped)
    }

    pub async fn structure_quantity<'a>(
        &self,
        template_positions: impl IntoIterator<Item = (&'a str, i32)>,
    ) -> Result<Option<i32>, Error> {
        let live_positions = self.option_qty_map().await?;
        Ok(structure_quantity(template_positions, &live_positions))
    }

    pub async fn reconcile_signed_positions<T>(
        &self,
        positions: &mut Vec<T>,
        symbol: impl Fn(&T) -> &str + Copy,
        set_signed_qty: impl FnMut(&mut T, i32),
    ) -> Result<(), Error> {
        let live_positions = self.option_qty_map().await?;
        reconcile_signed_positions(positions, &live_positions, symbol, set_signed_qty);
        Ok(())
    }

    pub async fn get(&self, symbol_or_asset_id: &str) -> Result<Position, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/positions/{}",
                request::validate_symbol_or_asset_id(symbol_or_asset_id)?
            ),
        )
        .with_operation("positions.get");

        self.inner
            .send_json::<Position>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn close_all(
        &self,
        request: CloseAllRequest,
    ) -> Result<Vec<ClosePositionResult>, Error> {
        let request = RequestParts::new(Method::DELETE, "/v2/positions")
            .with_operation("positions.close_all")
            .with_query(request.into_query());

        self.inner
            .send_json::<Vec<ClosePositionResult>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn close(
        &self,
        symbol_or_asset_id: &str,
        request: ClosePositionRequest,
    ) -> Result<ClosePositionBody, Error> {
        let request = RequestParts::new(
            Method::DELETE,
            format!(
                "/v2/positions/{}",
                request::validate_symbol_or_asset_id(symbol_or_asset_id)?
            ),
        )
        .with_operation("positions.close")
        .with_query(request.into_query());

        self.inner
            .send_json::<ClosePositionBody>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn exercise(
        &self,
        symbol_or_contract_id: &str,
    ) -> Result<ExercisePositionBody, Error> {
        let request = RequestParts::new(
            Method::POST,
            format!(
                "/v2/positions/{}/exercise",
                request::validate_symbol_or_contract_id(symbol_or_contract_id)?
            ),
        )
        .with_operation("positions.exercise");

        self.inner
            .send_json::<ExercisePositionBody>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn do_not_exercise(
        &self,
        symbol_or_contract_id: &str,
    ) -> Result<DoNotExerciseAccepted, Error> {
        let request = RequestParts::new(
            Method::POST,
            format!(
                "/v2/positions/{}/do-not-exercise",
                request::validate_symbol_or_contract_id(symbol_or_contract_id)?
            ),
        )
        .with_operation("positions.do_not_exercise");

        self.inner
            .send_no_content(request)
            .await
            .map(|_response: alpaca_http::HttpResponse<NoContent>| DoNotExerciseAccepted)
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for PositionsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
