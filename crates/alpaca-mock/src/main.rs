use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior, interval_at};

const LIMIT_ORDER_POLL_INTERVAL: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv_from_current_dir();

    let address = std::env::var("ALPACA_MOCK_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3847".to_owned())
        .parse::<SocketAddr>()?;
    let listener = TcpListener::bind(address).await?;
    let state = alpaca_mock::MockServerState::from_env()?;
    let poller = state
        .market_data_bridge()
        .is_some()
        .then(|| spawn_limit_order_poller(state.clone()));
    let app = alpaca_mock::build_app_with_state(state);

    println!("{} listening on http://{address}", alpaca_mock::BINARY_NAME);
    let server_result = axum::serve(listener, app).await;
    if let Some(poller) = poller {
        poller.abort();
        let _ = poller.await;
    }
    server_result?;
    Ok(())
}

fn spawn_limit_order_poller(state: alpaca_mock::MockServerState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = interval_at(
            Instant::now() + LIMIT_ORDER_POLL_INTERVAL,
            LIMIT_ORDER_POLL_INTERVAL,
        );
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            let report = state.poll_limit_orders_once().await;
            if let Some(error) = report.stock_market_data_error {
                eprintln!("stock limit-order poll failed: {error}");
            }
            if let Some(error) = report.option_market_data_error {
                eprintln!("option limit-order poll failed: {error}");
            }
        }
    })
}

fn load_dotenv_from_current_dir() {
    let Ok(current_dir) = std::env::current_dir() else {
        return;
    };
    if let Some(path) = find_dotenv_upward(&current_dir) {
        dotenvy::from_path(path).ok();
    }
}

fn find_dotenv_upward(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|candidate| candidate.join(".env"))
        .find(|path| path.exists())
}
