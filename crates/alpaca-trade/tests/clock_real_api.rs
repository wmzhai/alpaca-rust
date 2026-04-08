#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::Client;
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn clock_resource_reads_real_paper_clock() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.trade().expect("trade config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .base_url(service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);

    let clock = client
        .clock()
        .get()
        .await
        .expect("clock request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-clock", "get", &clock)
        .expect("clock sample should record");

    assert!(!clock.timestamp.is_empty());
    assert!(!clock.next_open.is_empty());
    assert!(!clock.next_close.is_empty());
}
