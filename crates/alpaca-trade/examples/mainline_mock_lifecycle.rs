use rust_decimal::Decimal;

use alpaca_data::Client as DataClient;
use alpaca_mock::{LiveMarketDataBridge, MockServerState, spawn_test_server_with_state};
use alpaca_trade::{
    Client,
    activities::ListRequest as ActivitiesListRequest,
    orders::{CreateRequest, OrderSide, OrderType, TimeInForce},
    positions::ClosePositionRequest,
};

const MAINLINE_SYMBOL: &str = "SPY";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_client = match DataClient::from_env() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Set ALPACA_DATA_* before running this example: {error}");
            return Ok(());
        }
    };

    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client));
    let server = spawn_test_server_with_state(state).await;
    let client = Client::builder()
        .api_key("example-mock-key")
        .secret_key("example-mock-secret")
        .base_url_str(&server.base_url)?
        .build()?;

    let account = client.account().get().await?;
    println!(
        "mock account {} is {}",
        account.account_number, account.status
    );

    let opened = client
        .orders()
        .create(CreateRequest {
            symbol: Some(MAINLINE_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("example-mainline-open".to_owned()),
            ..CreateRequest::default()
        })
        .await?;
    println!("opened order {} with status {:?}", opened.id, opened.status);

    let position = client.positions().get(MAINLINE_SYMBOL).await?;
    println!("position {} qty {}", position.symbol, position.qty);

    let fills = client
        .activities()
        .list(ActivitiesListRequest {
            activity_types: Some(vec!["FILL".to_owned()]),
            ..ActivitiesListRequest::default()
        })
        .await?;
    println!(
        "observed {} fill activities after the open order",
        fills.len()
    );

    let closed = client
        .positions()
        .close(MAINLINE_SYMBOL, ClosePositionRequest::default())
        .await?;
    println!("submitted close order {}", closed.id);

    Ok(())
}
