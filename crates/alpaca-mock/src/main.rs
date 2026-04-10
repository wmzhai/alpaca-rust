use std::net::SocketAddr;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let address = std::env::var("ALPACA_MOCK_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3847".to_owned())
        .parse::<SocketAddr>()?;
    let listener = TcpListener::bind(address).await?;
    let app = alpaca_mock::build_app_from_env()?;

    println!("{} listening on http://{address}", alpaca_mock::BINARY_NAME);
    axum::serve(listener, app).await?;
    Ok(())
}
