use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::{Error, client::ClientInner, pagination};

use super::{
    AuctionsRequest, AuctionsResponse, BarsRequest, BarsResponse, ConditionCodesRequest,
    ConditionCodesResponse, ExchangeCodesResponse, LatestBarsRequest, LatestBarsResponse,
    LatestQuotesRequest, LatestQuotesResponse, LatestTradesRequest, LatestTradesResponse,
    QuotesRequest, QuotesResponse, SnapshotsRequest, SnapshotsResponse, TradesRequest,
    TradesResponse,
};

#[derive(Clone)]
pub struct StocksClient {
    inner: Arc<ClientInner>,
}

impl StocksClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn bars(&self, request: BarsRequest) -> Result<BarsResponse, Error> {
        request.validate()?;
        self.get_json("stocks.bars", "/v2/stocks/bars", request.into_query())
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

    pub async fn auctions(&self, request: AuctionsRequest) -> Result<AuctionsResponse, Error> {
        request.validate()?;
        self.get_json(
            "stocks.auctions",
            "/v2/stocks/auctions",
            request.into_query(),
        )
        .await
    }

    pub async fn auctions_all(&self, request: AuctionsRequest) -> Result<AuctionsResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.auctions(request).await }
        })
        .await
    }

    pub async fn quotes(&self, request: QuotesRequest) -> Result<QuotesResponse, Error> {
        request.validate()?;
        self.get_json("stocks.quotes", "/v2/stocks/quotes", request.into_query())
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
        self.get_json("stocks.trades", "/v2/stocks/trades", request.into_query())
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
        self.get_json(
            "stocks.latest_bars",
            "/v2/stocks/bars/latest",
            request.into_query(),
        )
        .await
    }

    pub async fn latest_quotes(
        &self,
        request: LatestQuotesRequest,
    ) -> Result<LatestQuotesResponse, Error> {
        request.validate()?;
        self.get_json(
            "stocks.latest_quotes",
            "/v2/stocks/quotes/latest",
            request.into_query(),
        )
        .await
    }

    pub async fn latest_trades(
        &self,
        request: LatestTradesRequest,
    ) -> Result<LatestTradesResponse, Error> {
        request.validate()?;
        self.get_json(
            "stocks.latest_trades",
            "/v2/stocks/trades/latest",
            request.into_query(),
        )
        .await
    }

    pub async fn snapshots(&self, request: SnapshotsRequest) -> Result<SnapshotsResponse, Error> {
        request.validate()?;
        self.get_json(
            "stocks.snapshots",
            "/v2/stocks/snapshots",
            request.into_query(),
        )
        .await
    }

    pub async fn condition_codes(
        &self,
        request: ConditionCodesRequest,
    ) -> Result<ConditionCodesResponse, Error> {
        let path = format!("/v2/stocks/meta/conditions/{}", request.ticktype.as_str());
        self.get_json("stocks.condition_codes", path, request.into_query())
            .await
    }

    pub async fn exchange_codes(&self) -> Result<ExchangeCodesResponse, Error> {
        self.get_json(
            "stocks.exchange_codes",
            "/v2/stocks/meta/exchanges",
            Vec::new(),
        )
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

impl fmt::Debug for StocksClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StocksClient").finish()
    }
}
