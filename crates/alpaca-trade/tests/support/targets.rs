#![allow(dead_code)]

use std::{
    fs::{File, OpenOptions},
    sync::OnceLock,
    time::Duration,
};

use alpaca_data::Client as DataClient;
use alpaca_mock::{
    LiveMarketDataBridge, MockServerState, TestServer, spawn_test_server_with_state,
};
use alpaca_trade::Client as TradeClient;
use fs2::FileExt;
use tokio::sync::{Mutex, MutexGuard};

use crate::live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, PaperSessionState, SampleRecorder,
    can_submit_live_paper_orders, paper_market_session_state,
};

static LIVE_PAPER_MUTATION_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

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
    live_paper_session_state: Option<PaperSessionState>,
    _mock_server: Option<TestServer>,
}

pub(crate) struct LivePaperMutationGuard {
    _in_process_guard: MutexGuard<'static, ()>,
    lock_file: File,
}

impl Drop for LivePaperMutationGuard {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
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
        if self.is_mock() { "mock" } else { "paper" }
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
        self.live_paper_session_state.clone()
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

pub(crate) async fn build_trade_test_harness(target: TradeTestTarget) -> Option<TradeTestHarness> {
    let env = LiveTestEnv::load().expect("live test environment should load");

    match target {
        TradeTestTarget::LivePaper => build_live_paper_harness(env).await,
        TradeTestTarget::Mock => build_mock_harness(env).await,
    }
}

pub(crate) async fn lock_live_paper_account() -> LivePaperMutationGuard {
    let in_process_guard = LIVE_PAPER_MUTATION_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;
    let lock_file = tokio::task::spawn_blocking(open_live_paper_lock_file)
        .await
        .expect("live paper lock task should join")
        .expect("live paper lock file should open and lock");

    LivePaperMutationGuard {
        _in_process_guard: in_process_guard,
        lock_file,
    }
}

fn open_live_paper_lock_file() -> std::io::Result<File> {
    let path = std::env::temp_dir().join("alpaca-rust-live-paper-account.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(file)
}

async fn build_live_paper_harness(env: LiveTestEnv) -> Option<TradeTestHarness> {
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
    let recorder = SampleRecorder::from_live_env(&env);
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let session_state =
        match paper_market_session_state(&probe, &trade_service, Some(&recorder)).await {
            Ok(session_state) => session_state,
            Err(error) => {
                eprintln!("skipping live paper test: market session probe failed: {error}");
                return None;
            }
        };
    if !can_submit_live_paper_orders(&session_state) {
        eprintln!("skipping live paper test: market session is unavailable");
        return None;
    }

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

    Some(TradeTestHarness {
        target: TradeTestTarget::LivePaper,
        trade_client,
        data_client,
        recorder,
        live_paper_session_state: Some(session_state),
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
    let state = MockServerState::new()
        .with_market_data_bridge(LiveMarketDataBridge::new(data_client.clone()));
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
        live_paper_session_state: None,
        _mock_server: Some(server),
    })
}
