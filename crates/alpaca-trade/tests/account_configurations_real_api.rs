#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{Client, account_configurations::UpdateRequest};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn account_configurations_resource_reads_and_updates_real_paper_configurations() {
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

    let original = client
        .account_configurations()
        .get()
        .await
        .expect("account configurations request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-account-configurations", "get", &original)
        .expect("account configurations sample should record");

    let original_trade_confirm_email = original
        .trade_confirm_email
        .clone()
        .expect("real paper account configuration should include trade_confirm_email");
    let next_trade_confirm_email = if original_trade_confirm_email == "all" {
        "none"
    } else {
        "all"
    };

    let updated = client
        .account_configurations()
        .update(UpdateRequest {
            trade_confirm_email: Some(next_trade_confirm_email.to_owned()),
            ..UpdateRequest::default()
        })
        .await
        .expect("account configurations update should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-account-configurations", "update", &updated)
        .expect("account configurations update sample should record");

    assert_eq!(
        updated.trade_confirm_email.as_deref(),
        Some(next_trade_confirm_email)
    );

    let restored = client
        .account_configurations()
        .update(UpdateRequest {
            trade_confirm_email: Some(original_trade_confirm_email.clone()),
            ..UpdateRequest::default()
        })
        .await
        .expect("account configurations restore should succeed against real paper API");

    assert_eq!(
        restored.trade_confirm_email.as_deref(),
        Some(original_trade_confirm_email.as_str())
    );
}
