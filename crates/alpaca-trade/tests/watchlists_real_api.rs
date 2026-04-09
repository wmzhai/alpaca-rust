#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_trade::{
    Client,
    watchlists::{AddAssetRequest, CreateRequest, UpdateRequest},
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn watchlists_resource_covers_id_and_name_flows_against_real_paper_api() {
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
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_millis();
    let primary_name = format!("phase1-watchlist-primary-{nonce}");
    let primary_renamed = format!("phase1-watchlist-primary-renamed-{nonce}");
    let secondary_name = format!("phase1-watchlist-secondary-{nonce}");
    let secondary_renamed = format!("phase1-watchlist-secondary-renamed-{nonce}");

    let primary = client
        .watchlists()
        .create(CreateRequest {
            name: primary_name.clone(),
            symbols: Some(vec!["AAPL".to_owned(), "MSFT".to_owned()]),
        })
        .await
        .expect("watchlist create should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-watchlists", "create", &primary)
        .expect("watchlist create sample should record");

    let watchlists = client
        .watchlists()
        .list()
        .await
        .expect("watchlists list should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-watchlists", "list", &watchlists)
        .expect("watchlist list sample should record");
    assert!(
        watchlists
            .iter()
            .any(|watchlist| watchlist.id == primary.id)
    );

    let fetched_by_id = client
        .watchlists()
        .get_by_id(&primary.id)
        .await
        .expect("watchlist get by id should succeed against real paper API");
    assert_eq!(fetched_by_id.id, primary.id);

    let updated_by_id = client
        .watchlists()
        .update_by_id(
            &primary.id,
            UpdateRequest {
                name: primary_renamed.clone(),
                symbols: Some(vec!["AAPL".to_owned()]),
            },
        )
        .await
        .expect("watchlist update by id should succeed against real paper API");
    assert_eq!(updated_by_id.name, primary_renamed);

    let added_by_id = client
        .watchlists()
        .add_asset_by_id(
            &primary.id,
            AddAssetRequest {
                symbol: "SPY".to_owned(),
            },
        )
        .await
        .expect("watchlist add asset by id should succeed against real paper API");
    assert!(added_by_id.assets.iter().any(|asset| asset.symbol == "SPY"));

    let removed_symbol = client
        .watchlists()
        .delete_symbol_by_id(&primary.id, "AAPL")
        .await
        .expect("watchlist delete symbol by id should succeed against real paper API");
    assert!(
        removed_symbol
            .assets
            .iter()
            .all(|asset| asset.symbol != "AAPL")
    );

    client
        .watchlists()
        .delete_by_id(&primary.id)
        .await
        .expect("watchlist delete by id should succeed against real paper API");

    let secondary = client
        .watchlists()
        .create(CreateRequest {
            name: secondary_name.clone(),
            symbols: Some(vec!["QQQ".to_owned()]),
        })
        .await
        .expect("second watchlist create should succeed against real paper API");

    let fetched_by_name = client
        .watchlists()
        .get_by_name(&secondary_name)
        .await
        .expect("watchlist get by name should succeed against real paper API");
    assert_eq!(fetched_by_name.id, secondary.id);

    let updated_by_name = client
        .watchlists()
        .update_by_name(
            &secondary_name,
            UpdateRequest {
                name: secondary_renamed.clone(),
                symbols: Some(vec!["IWM".to_owned()]),
            },
        )
        .await
        .expect("watchlist update by name should succeed against real paper API");
    assert_eq!(updated_by_name.name, secondary_renamed);

    let added_by_name = client
        .watchlists()
        .add_asset_by_name(
            &secondary_renamed,
            AddAssetRequest {
                symbol: "DIA".to_owned(),
            },
        )
        .await
        .expect("watchlist add asset by name should succeed against real paper API");
    assert!(
        added_by_name
            .assets
            .iter()
            .any(|asset| asset.symbol == "DIA")
    );

    client
        .watchlists()
        .delete_by_name(&secondary_renamed)
        .await
        .expect("watchlist delete by name should succeed against real paper API");
}
