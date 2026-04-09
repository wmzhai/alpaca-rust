#![allow(dead_code)]

use std::time::Duration;

use alpaca_data::Client as DataClient;
use alpaca_mock::{
    LiveMarketDataBridge, MockServerState, TestServer, spawn_test_server_with_state,
};
use alpaca_trade::Client as TradeClient;

use crate::live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, PaperSessionState, SampleRecorder, ServiceConfig,
    can_submit_live_paper_orders, paper_market_session_state,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TradeTestTarget {
    LivePaper,
    Mock,
}

impl TradeTestTarget {
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::LivePaper => "live paper",
            Self::Mock => "mock",
        }
    }

    #[must_use]
    pub(crate) fn poll_attempts(self) -> usize {
        match self {
            Self::LivePaper => 20,
            Self::Mock => 10,
        }
    }

    #[must_use]
    pub(crate) fn poll_interval(self) -> Duration {
        match self {
            Self::LivePaper => Duration::from_secs(1),
            Self::Mock => Duration::from_millis(250),
        }
    }
}

pub(crate) struct TradeTestHarness {
    target: TradeTestTarget,
    trade_client: TradeClient,
    data_client: DataClient,
    recorder: SampleRecorder,
    live_trade_service: Option<ServiceConfig>,
    _mock_server: Option<TestServer>,
}

impl TradeTestHarness {
    #[must_use]
    pub(crate) fn is_live_paper(&self) -> bool {
        self.target == TradeTestTarget::LivePaper
    }

    #[must_use]
    pub(crate) fn is_mock(&self) -> bool {
        self.target == TradeTestTarget::Mock
    }

    #[must_use]
    pub(crate) fn label(&self) -> &'static str {
        self.target.label()
    }

    #[must_use]
    pub(crate) fn slug(&self) -> &'static str {
        if self.is_mock() {
            "mock"
        } else {
            "paper"
        }
    }

    #[must_use]
    pub(crate) fn trade_client(&self) -> &TradeClient {
        &self.trade_client
    }

    #[must_use]
    pub(crate) fn data_client(&self) -> &DataClient {
        &self.data_client
    }

    #[must_use]
    pub(crate) fn recorder(&self) -> Option<&SampleRecorder> {
        if self.is_live_paper() {
            Some(&self.recorder)
        } else {
            None
        }
    }

    #[must_use]
    pub(crate) fn poll_attempts(&self) -> usize {
        self.target.poll_attempts()
    }

    #[must_use]
    pub(crate) fn poll_interval(&self) -> Duration {
        self.target.poll_interval()
    }

    pub(crate) async fn live_paper_session_state(&self) -> Option<PaperSessionState> {
        if !self.is_live_paper() {
            return None;
        }

        let probe = LiveHttpProbe::new().expect("live probe should build");
        let trade_service = self
            .live_trade_service
            .as_ref()
            .expect("live paper harness should retain the trade service");
        Some(
            paper_market_session_state(&probe, trade_service, self.recorder())
                .await
                .expect("paper clock and calendar should be readable"),
        )
    }

    pub(crate) async fn should_skip_live_market_session(&self, scenario: &str) -> bool {
        let Some(state) = self.live_paper_session_state().await else {
            return false;
        };
        if can_submit_live_paper_orders(&state) {
            return false;
        }

        eprintln!(
            "skipping {} {}: market session is unavailable",
            self.label(),
            scenario
        );
        true
    }
}

pub(crate) async fn build_trade_test_harness(
    target: TradeTestTarget,
) -> Option<TradeTestHarness> {
    let env = LiveTestEnv::load().expect("live test environment should load");

    match target {
        TradeTestTarget::LivePaper => build_live_paper_harness(env),
        TradeTestTarget::Mock => build_mock_harness(env).await,
    }
}

fn build_live_paper_harness(env: LiveTestEnv) -> Option<TradeTestHarness> {
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping live paper test: {reason}");
        return None;
    }
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping live paper test: {reason}");
        return None;
    }

    let trade_service = env.trade().expect("trade config should exist").clone();
    let data_service = env.data().expect("data config should exist").clone();
    let trade_client = TradeClient::builder()
        .credentials(trade_service.credentials().clone())
        .base_url(trade_service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);

    Some(TradeTestHarness {
        target: TradeTestTarget::LivePaper,
        trade_client,
        data_client,
        recorder,
        live_trade_service: Some(trade_service),
        _mock_server: None,
    })
}

async fn build_mock_harness(env: LiveTestEnv) -> Option<TradeTestHarness> {
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock test: {reason}");
        return None;
    }

    let data_service = env.data().expect("data config should exist").clone();
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client.clone()));
    let server = spawn_test_server_with_state(state).await;
    let trade_client = TradeClient::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");
    let recorder = SampleRecorder::from_live_env(&env);

    Some(TradeTestHarness {
        target: TradeTestTarget::Mock,
        trade_client,
        data_client,
        recorder,
        live_trade_service: None,
        _mock_server: Some(server),
    })
}
