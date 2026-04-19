#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, SampleRecorder, discover_active_option_contract,
    paper_market_session_state,
};

#[tokio::test]
async fn live_support_real_api_reads_data_and_paper_endpoints() {
    let env = LiveTestEnv::load().expect("live test environment should load");

    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let probe = LiveHttpProbe::new().expect("probe should build");
    let recorder = SampleRecorder::from_live_env(&env);

    let contract = discover_active_option_contract(
        env.data().expect("data config should exist"),
        Some(&recorder),
        "SPY",
        5,
    )
    .await
    .expect("real options snapshots should return at least one contract");
    assert!(!contract.symbol.is_empty());
    assert!(contract.reference_timestamp.is_some());

    let session = paper_market_session_state(
        &probe,
        env.trade().expect("trade config should exist"),
        Some(&recorder),
    )
    .await
    .expect("paper clock and calendar should be readable");
    assert!(!session.clock.timestamp.is_empty());
}
