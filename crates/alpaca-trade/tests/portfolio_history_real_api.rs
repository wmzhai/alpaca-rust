#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{
    Client,
    portfolio_history::GetRequest,
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn portfolio_history_resource_reads_real_paper_history_window() {
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

    let history = client
        .portfolio_history()
        .get(GetRequest {
            period: Some("1D".to_owned()),
            timeframe: Some("1D".to_owned()),
            ..GetRequest::default()
        })
        .await
        .expect("portfolio history request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-portfolio-history", "get", &history)
        .expect("portfolio history sample should record");

    assert!(!history.timestamp.is_empty());
    assert_eq!(history.timestamp.len(), history.equity.len());
    assert_eq!(history.timestamp.len(), history.profit_loss.len());
    assert_eq!(history.timestamp.len(), history.profit_loss_pct.len());
    assert!(!history.timeframe.is_empty());
}
