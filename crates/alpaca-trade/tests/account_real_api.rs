#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::Client;
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn account_resource_reads_real_paper_account() {
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
        .expect("client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);

    let account = client
        .account()
        .get()
        .await
        .expect("account request should succeed against real paper API");

    recorder
        .record_json("alpaca-trade-account", "get", &account)
        .expect("account sample should record");

    assert!(!account.id.trim().is_empty());
    assert!(!account.account_number.trim().is_empty());
    assert!(!account.status.trim().is_empty());
    assert!(account.cash.is_some());
}
