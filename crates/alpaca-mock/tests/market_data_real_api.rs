#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use live_support::{AlpacaService, LiveTestEnv, SampleRecorder, discover_active_option_contract};

use alpaca_data::Client;
use alpaca_mock::{DEFAULT_STOCK_SYMBOL, LiveMarketDataBridge};

#[tokio::test]
async fn market_data_bridge_reads_real_equity_and_option_snapshots() {
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

    let contract = discover_active_option_contract(service, Some(&recorder), "SPY", 8)
        .await
        .expect("should discover a live option contract");
    let option = bridge
        .option_snapshot(&contract.symbol)
        .await
        .expect("option snapshot should read from real API");
    recorder
        .record_json(
            "alpaca-mock-market-data",
            "option-snapshot",
            &serde_json::json!({
                "symbol": contract.symbol,
                "asset_class": option.asset_class,
                "bid": option.bid.to_string(),
                "ask": option.ask.to_string(),
                "previous_close": option.previous_close.map(|value| value.to_string()),
            }),
        )
        .expect("option snapshot sample should record");
    assert_eq!(option.asset_class, "us_option");
}
