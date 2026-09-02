use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::{Error, client::ClientInner, pagination};

use super::{
    BarsRequest, BarsResponse, LatestBarsRequest, LatestBarsResponse, LatestOrderbooksRequest,
    LatestOrderbooksResponse, LatestQuotesRequest, LatestQuotesResponse, LatestTradesRequest,
    LatestTradesResponse, QuotesRequest, QuotesResponse, SnapshotsRequest, SnapshotsResponse,
    TradesRequest, TradesResponse,
};

#[derive(Clone)]
pub struct CryptoClient {
    inner: Arc<ClientInner>,
}

impl CryptoClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn bars(&self, request: BarsRequest) -> Result<BarsResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/bars", request.location);
        self.get_json("crypto.bars", path, request.into_query())
            .await
    }

    pub async fn bars_all(&self, request: BarsRequest) -> Result<BarsResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.bars(request).await }
        })
        .await
    }

    pub async fn quotes(&self, request: QuotesRequest) -> Result<QuotesResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/quotes", request.location);
        self.get_json("crypto.quotes", path, request.into_query())
            .await
    }

    pub async fn quotes_all(&self, request: QuotesRequest) -> Result<QuotesResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.quotes(request).await }
        })
        .await
    }

    pub async fn trades(&self, request: TradesRequest) -> Result<TradesResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/trades", request.location);
        self.get_json("crypto.trades", path, request.into_query())
            .await
    }

    pub async fn trades_all(&self, request: TradesRequest) -> Result<TradesResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.trades(request).await }
        })
        .await
    }

    pub async fn latest_bars(
        &self,
        request: LatestBarsRequest,
    ) -> Result<LatestBarsResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/latest/bars", request.location);
        self.get_json("crypto.latest_bars", path, request.into_query())
            .await
    }

    pub async fn latest_quotes(
        &self,
        request: LatestQuotesRequest,
    ) -> Result<LatestQuotesResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/latest/quotes", request.location);
        self.get_json("crypto.latest_quotes", path, request.into_query())
            .await
    }

    pub async fn latest_trades(
        &self,
        request: LatestTradesRequest,
    ) -> Result<LatestTradesResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/latest/trades", request.location);
        self.get_json("crypto.latest_trades", path, request.into_query())
            .await
    }

    pub async fn latest_orderbooks(
        &self,
        request: LatestOrderbooksRequest,
    ) -> Result<LatestOrderbooksResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/latest/orderbooks", request.location);
        self.get_json("crypto.latest_orderbooks", path, request.into_query())
            .await
    }

    pub async fn snapshots(&self, request: SnapshotsRequest) -> Result<SnapshotsResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta3/crypto/{}/snapshots", request.location);
        self.get_json("crypto.snapshots", path, request.into_query())
            .await
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }

    async fn get_json<Response>(
        &self,
        operation: &'static str,
        path: impl Into<String>,
        query: Vec<(String, String)>,
    ) -> Result<Response, Error>
    where
        Response: DeserializeOwned,
    {
        let request = RequestParts::new(Method::GET, path.into())
            .with_operation(operation)
            .with_query(query);

        self.inner
            .send_json::<Response>(request)
            .await
            .map(|response| response.into_body())
    }
}

impl fmt::Debug for CryptoClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CryptoClient").finish()
    }
}
