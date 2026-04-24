use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_dotenv_from_current_dir();

    let address = std::env::var("ALPACA_MOCK_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3847".to_owned())
        .parse::<SocketAddr>()?;
    let listener = TcpListener::bind(address).await?;
    let app = alpaca_mock::build_app_from_env()?;

    println!("{} listening on http://{address}", alpaca_mock::BINARY_NAME);
    axum::serve(listener, app).await?;
    Ok(())
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
