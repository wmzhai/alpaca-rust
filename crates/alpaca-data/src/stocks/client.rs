use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::{Error, client::ClientInner, pagination};
use crate::stocks::display_symbol;

use super::{
    AuctionsRequest, AuctionsResponse, AuctionsSingleRequest, AuctionsSingleResponse, BarsRequest,
    BarsResponse, BarsSingleRequest, BarsSingleResponse, ConditionCodesRequest,
    ConditionCodesResponse, ExchangeCodesResponse, LatestBarRequest, LatestBarResponse,
    LatestBarsRequest, LatestBarsResponse, LatestQuoteRequest, LatestQuoteResponse,
    LatestQuotesRequest, LatestQuotesResponse, LatestTradeRequest, LatestTradeResponse,
    LatestTradesRequest, LatestTradesResponse, QuotesRequest, QuotesResponse, QuotesSingleRequest,
    QuotesSingleResponse, SnapshotRequest, SnapshotResponse, SnapshotsRequest, SnapshotsResponse,
    TradesRequest, TradesResponse, TradesSingleRequest, TradesSingleResponse,
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

    pub async fn bars_single(
        &self,
        request: BarsSingleRequest,
    ) -> Result<BarsSingleResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/bars", display_symbol(&request.symbol));
        self.get_json("stocks.bars_single", path, request.into_query())
            .await
    }

    pub async fn bars_single_all(
        &self,
        request: BarsSingleRequest,
    ) -> Result<BarsSingleResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.bars_single(request).await }
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

    pub async fn auctions_single(
        &self,
        request: AuctionsSingleRequest,
    ) -> Result<AuctionsSingleResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/auctions", display_symbol(&request.symbol));
        self.get_json("stocks.auctions_single", path, request.into_query())
            .await
    }

    pub async fn auctions_single_all(
        &self,
        request: AuctionsSingleRequest,
    ) -> Result<AuctionsSingleResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.auctions_single(request).await }
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

    pub async fn quotes_single(
        &self,
        request: QuotesSingleRequest,
    ) -> Result<QuotesSingleResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/quotes", display_symbol(&request.symbol));
        self.get_json("stocks.quotes_single", path, request.into_query())
            .await
    }

    pub async fn quotes_single_all(
        &self,
        request: QuotesSingleRequest,
    ) -> Result<QuotesSingleResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.quotes_single(request).await }
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

    pub async fn trades_single(
        &self,
        request: TradesSingleRequest,
    ) -> Result<TradesSingleResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/trades", display_symbol(&request.symbol));
        self.get_json("stocks.trades_single", path, request.into_query())
            .await
    }

    pub async fn trades_single_all(
        &self,
        request: TradesSingleRequest,
    ) -> Result<TradesSingleResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.trades_single(request).await }
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

    pub async fn latest_bar(&self, request: LatestBarRequest) -> Result<LatestBarResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/bars/latest", display_symbol(&request.symbol));
        self.get_json("stocks.latest_bar", path, request.into_query())
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

    pub async fn latest_quote(
        &self,
        request: LatestQuoteRequest,
    ) -> Result<LatestQuoteResponse, Error> {
        request.validate()?;
        let path = format!(
            "/v2/stocks/{}/quotes/latest",
            display_symbol(&request.symbol)
        );
        self.get_json("stocks.latest_quote", path, request.into_query())
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

    pub async fn latest_trade(
        &self,
        request: LatestTradeRequest,
    ) -> Result<LatestTradeResponse, Error> {
        request.validate()?;
        let path = format!(
            "/v2/stocks/{}/trades/latest",
            display_symbol(&request.symbol)
        );
        self.get_json("stocks.latest_trade", path, request.into_query())
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

    pub async fn snapshot(&self, request: SnapshotRequest) -> Result<SnapshotResponse, Error> {
        request.validate()?;
        let path = format!("/v2/stocks/{}/snapshot", display_symbol(&request.symbol));
        self.get_json("stocks.snapshot", path, request.into_query())
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
        f.debug_struct("StocksClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
