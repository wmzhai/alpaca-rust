#![forbid(unsafe_code)]

mod auth;

pub mod app;
pub mod handlers;
pub mod state;

use tokio::{net::TcpListener, task::JoinHandle};

pub use app::{build_app, build_app_from_env, build_app_with_state};
pub use state::{
    AdminStateResponse, DEFAULT_STOCK_SYMBOL, InjectedHttpFault, InstrumentSnapshot,
    LiveMarketDataBridge, MarketDataBridgeError, MockServerState,
};

pub const BINARY_NAME: &str = "alpaca-mock";

#[derive(Debug)]
pub struct TestServer {
    pub base_url: String,
    _task: JoinHandle<()>,
}

pub async fn spawn_test_server() -> TestServer {
    spawn_test_server_with_state(MockServerState::new()).await
}

pub async fn spawn_test_server_with_state(state: MockServerState) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("local addr should exist");
    let app = build_app_with_state(state);

    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });

    TestServer {
        base_url: format!("http://{address}"),
        _task: task,
    }
}
