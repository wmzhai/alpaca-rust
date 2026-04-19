#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

use alpaca_data::Client;
use alpaca_mock::{DEFAULT_STOCK_SYMBOL, LiveMarketDataBridge};

#[tokio::test]
async fn market_data_bridge_reads_real_equity_snapshot() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.data().expect("data config should exist");
    let bridge = LiveMarketDataBridge::new(
        Client::builder()
            .credentials(service.credentials().clone())
            .build()
            .expect("data client should build from live service config"),
    );
    let recorder = SampleRecorder::from_live_env(&env);

    let equity = bridge
        .equity_snapshot(DEFAULT_STOCK_SYMBOL)
        .await
        .expect("equity snapshot should read from real API");
    recorder
        .record_json(
            "alpaca-mock-market-data",
            "equity-snapshot",
            &serde_json::json!({
                "symbol": DEFAULT_STOCK_SYMBOL,
                "asset_class": equity.asset_class,
                "bid": equity.bid.to_string(),
                "ask": equity.ask.to_string(),
                "previous_close": equity.previous_close.map(|value| value.to_string()),
            }),
        )
        .expect("equity snapshot sample should record");
    assert_eq!(equity.asset_class, "us_equity");
}
