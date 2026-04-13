use std::fmt;
use std::sync::Arc;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::{Error, client::ClientInner, pagination};

use super::response::merge_snapshot_page;
use super::{
    BarsRequest, BarsResponse, ChainRequest, ChainResponse, ConditionCodesRequest,
    ConditionCodesResponse, ExchangeCodesResponse, LatestQuotesRequest, LatestQuotesResponse,
    LatestTradesRequest, LatestTradesResponse, SnapshotsRequest, SnapshotsResponse, TradesRequest,
    TradesResponse,
};

const MAX_SNAPSHOT_SYMBOLS_PER_REQUEST: usize = 100;

#[derive(Clone)]
pub struct OptionsClient {
    inner: Arc<ClientInner>,
}

impl OptionsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn bars(&self, request: BarsRequest) -> Result<BarsResponse, Error> {
        request.validate()?;
        self.get_json(
            "options.bars",
            "/v1beta1/options/bars",
            request.into_query(),
        )
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

    pub async fn trades(&self, request: TradesRequest) -> Result<TradesResponse, Error> {
        request.validate()?;
        self.get_json(
            "options.trades",
            "/v1beta1/options/trades",
            request.into_query(),
        )
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

    pub async fn latest_quotes(
        &self,
        request: LatestQuotesRequest,
    ) -> Result<LatestQuotesResponse, Error> {
        request.validate()?;
        self.get_json(
            "options.latest_quotes",
            "/v1beta1/options/quotes/latest",
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
            "options.latest_trades",
            "/v1beta1/options/trades/latest",
            request.into_query(),
        )
        .await
    }

    pub async fn snapshots(&self, request: SnapshotsRequest) -> Result<SnapshotsResponse, Error> {
        request.validate()?;
        self.get_json(
            "options.snapshots",
            "/v1beta1/options/snapshots",
            request.into_query(),
        )
        .await
    }

    pub async fn snapshots_all(
        &self,
        request: SnapshotsRequest,
    ) -> Result<SnapshotsResponse, Error> {
        request.validate_all()?;

        let mut combined = SnapshotsResponse::default();
        for batch in request.batches(MAX_SNAPSHOT_SYMBOLS_PER_REQUEST) {
            let client = self.clone();
            let next = pagination::collect_all(batch, move |request| {
                let client = client.clone();
                async move { client.snapshots(request).await }
            })
            .await?;

            merge_snapshot_page(
                "options.snapshots_all",
                &mut combined.snapshots,
                next.snapshots,
            )?;
        }

        Ok(combined)
    }

    pub async fn chain(&self, request: ChainRequest) -> Result<ChainResponse, Error> {
        request.validate()?;
        let path = format!("/v1beta1/options/snapshots/{}", request.path_symbol());
        self.get_json("options.chain", path, request.into_query())
            .await
    }

    pub async fn chain_all(&self, request: ChainRequest) -> Result<ChainResponse, Error> {
        let client = self.clone();
        pagination::collect_all(request, move |request| {
            let client = client.clone();
            async move { client.chain(request).await }
        })
        .await
    }

    pub async fn condition_codes(
        &self,
        request: ConditionCodesRequest,
    ) -> Result<ConditionCodesResponse, Error> {
        let path = format!(
            "/v1beta1/options/meta/conditions/{}",
            request.ticktype.as_str()
        );
        self.get_json("options.condition_codes", path, Vec::new())
            .await
    }

    pub async fn exchange_codes(&self) -> Result<ExchangeCodesResponse, Error> {
        self.get_json(
            "options.exchange_codes",
            "/v1beta1/options/meta/exchanges",
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

impl fmt::Debug for OptionsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OptionsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
