use std::fmt;
use std::sync::Arc;

use alpaca_http::{NoContent, RequestParts};
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    watchlists::{
        AddAssetRequest, CreateRequest, UpdateRequest, Watchlist, WatchlistSummary, request,
    },
};

#[derive(Clone)]
pub struct WatchlistsClient {
    inner: Arc<ClientInner>,
}

impl WatchlistsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self) -> Result<Vec<WatchlistSummary>, Error> {
        let request =
            RequestParts::new(Method::GET, "/v2/watchlists").with_operation("watchlists.list");

        self.inner
            .send_json::<Vec<WatchlistSummary>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn create(&self, request: CreateRequest) -> Result<Watchlist, Error> {
        let request = RequestParts::new(Method::POST, "/v2/watchlists")
            .with_operation("watchlists.create")
            .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn get_by_id(&self, watchlist_id: &str) -> Result<Watchlist, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/watchlists/{}",
                request::validate_watchlist_id(watchlist_id)?
            ),
        )
        .with_operation("watchlists.get_by_id");

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn update_by_id(
        &self,
        watchlist_id: &str,
        request: UpdateRequest,
    ) -> Result<Watchlist, Error> {
        let request = RequestParts::new(
            Method::PUT,
            format!(
                "/v2/watchlists/{}",
                request::validate_watchlist_id(watchlist_id)?
            ),
        )
        .with_operation("watchlists.update_by_id")
        .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn delete_by_id(&self, watchlist_id: &str) -> Result<NoContent, Error> {
        let request = RequestParts::new(
            Method::DELETE,
            format!(
                "/v2/watchlists/{}",
                request::validate_watchlist_id(watchlist_id)?
            ),
        )
        .with_operation("watchlists.delete_by_id");

        self.inner
            .send_no_content(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn add_asset_by_id(
        &self,
        watchlist_id: &str,
        request: AddAssetRequest,
    ) -> Result<Watchlist, Error> {
        let request = RequestParts::new(
            Method::POST,
            format!(
                "/v2/watchlists/{}",
                request::validate_watchlist_id(watchlist_id)?
            ),
        )
        .with_operation("watchlists.add_asset_by_id")
        .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn delete_symbol_by_id(
        &self,
        watchlist_id: &str,
        symbol: &str,
    ) -> Result<Watchlist, Error> {
        let request = RequestParts::new(
            Method::DELETE,
            format!(
                "/v2/watchlists/{}/{}",
                request::validate_watchlist_id(watchlist_id)?,
                request::validate_symbol(symbol)?
            ),
        )
        .with_operation("watchlists.delete_symbol_by_id");

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Watchlist, Error> {
        let request = RequestParts::new(Method::GET, "/v2/watchlists:by_name")
            .with_operation("watchlists.get_by_name")
            .with_query(request::name_query(name)?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn update_by_name(
        &self,
        name: &str,
        request: UpdateRequest,
    ) -> Result<Watchlist, Error> {
        let request = RequestParts::new(Method::PUT, "/v2/watchlists:by_name")
            .with_operation("watchlists.update_by_name")
            .with_query(request::name_query(name)?)
            .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn add_asset_by_name(
        &self,
        name: &str,
        request: AddAssetRequest,
    ) -> Result<Watchlist, Error> {
        let request = RequestParts::new(Method::POST, "/v2/watchlists:by_name")
            .with_operation("watchlists.add_asset_by_name")
            .with_query(request::name_query(name)?)
            .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Watchlist>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn delete_by_name(&self, name: &str) -> Result<NoContent, Error> {
        let request = RequestParts::new(Method::DELETE, "/v2/watchlists:by_name")
            .with_operation("watchlists.delete_by_name")
            .with_query(request::name_query(name)?);

        self.inner
            .send_no_content(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for WatchlistsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchlistsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
