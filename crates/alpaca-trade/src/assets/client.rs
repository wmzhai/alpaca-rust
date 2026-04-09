use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    assets::{
        Asset, ListRequest, UsCorporatesRequest, UsCorporatesResponse, UsTreasuriesRequest,
        UsTreasuriesResponse,
    },
};

#[derive(Clone)]
pub struct AssetsClient {
    inner: Arc<ClientInner>,
}

impl AssetsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Asset>, Error> {
        let request = RequestParts::new(Method::GET, "/v2/assets")
            .with_operation("assets.list")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<Vec<Asset>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn get(&self, symbol_or_asset_id: &str) -> Result<Asset, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/assets/{}",
                super::request::validate_symbol_or_asset_id(symbol_or_asset_id)?
            ),
        )
        .with_operation("assets.get");

        self.inner
            .send_json::<Asset>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn fixed_income_us_corporates(
        &self,
        request: UsCorporatesRequest,
    ) -> Result<UsCorporatesResponse, Error> {
        let request = RequestParts::new(Method::GET, "/v2/assets/fixed_income/us_corporates")
            .with_operation("assets.fixed_income_us_corporates")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<UsCorporatesResponse>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn fixed_income_us_treasuries(
        &self,
        request: UsTreasuriesRequest,
    ) -> Result<UsTreasuriesResponse, Error> {
        let request = RequestParts::new(Method::GET, "/v2/assets/fixed_income/us_treasuries")
            .with_operation("assets.fixed_income_us_treasuries")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<UsTreasuriesResponse>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for AssetsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
