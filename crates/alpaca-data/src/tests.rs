#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use reqwest::Method;
use serde_json::Value;

use crate::client::Client;
use alpaca_http::RequestParts;
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn client_foundation_uses_real_data_api() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.data().expect("data config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .base_url(service.base_url().clone())
        .build()
        .expect("client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);
    let request = RequestParts::new(Method::GET, "/v2/stocks/bars/latest")
        .with_operation("stocks.latest_bars.foundation")
        .with_query(vec![
            ("symbols".to_owned(), "AAPL".to_owned()),
            ("feed".to_owned(), "iex".to_owned()),
        ]);

    let response = client
        .inner
        .send_json::<Value>(request)
        .await
        .expect("foundation client should read latest stock bars from real API");

    recorder
        .record_json("alpaca-data", "stocks-latest-bars-foundation", response.body())
        .expect("sample recording should not fail");

    let bars = response.body()["bars"].as_object().expect("bars object should exist");
    assert!(bars.contains_key("AAPL"));
    assert_eq!(response.meta().status(), 200);
}
