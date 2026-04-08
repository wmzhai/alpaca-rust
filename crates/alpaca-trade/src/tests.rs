#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_http::RequestParts;
use reqwest::Method;
use serde_json::Value;

use crate::Client;
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn client_foundation_uses_real_trade_paper_api() {
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
    let request =
        RequestParts::new(Method::GET, "/v2/clock").with_operation("trade.clock.foundation");

    let response = client
        .inner
        .send_json::<Value>(request)
        .await
        .expect("foundation client should read paper clock from real API");

    recorder
        .record_json("alpaca-trade", "clock-foundation", response.body())
        .expect("sample recording should not fail");

    let body = response.body();
    assert!(
        body["timestamp"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(body["is_open"].is_boolean());
    assert_eq!(response.meta().status(), 200);
}
