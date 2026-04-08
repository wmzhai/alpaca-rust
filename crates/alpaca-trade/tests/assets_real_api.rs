#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{Client, assets::ListRequest};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn assets_resource_reads_real_paper_assets_list_and_single_asset() {
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

    let assets = client
        .assets()
        .list(ListRequest {
            status: Some("active".to_owned()),
            asset_class: Some("us_equity".to_owned()),
            exchange: None,
            attributes: Some(vec!["has_options".to_owned()]),
        })
        .await
        .expect("assets list request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-assets", "list", &assets)
        .expect("assets list sample should record");
    assert!(!assets.is_empty());

    let by_symbol = client
        .assets()
        .get(&assets[0].symbol)
        .await
        .expect("asset get by symbol should succeed against real paper API");
    let by_id = client
        .assets()
        .get(&assets[0].id)
        .await
        .expect("asset get by id should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-assets", "get", &by_symbol)
        .expect("asset get sample should record");

    assert_eq!(by_symbol.id, by_id.id);
    assert_eq!(by_symbol.symbol, assets[0].symbol);
}
