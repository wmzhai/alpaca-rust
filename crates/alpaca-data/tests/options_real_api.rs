#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    options::{ChainRequest, SnapshotsRequest, options_underlying_symbol, preferred_feed},
};
use live_support::{AlpacaService, LiveTestEnv, discover_option_contracts};

#[tokio::test]
#[ignore = "slow live batching audit; run explicitly when validating option snapshot batching"]
async fn options_snapshots_all_absorbs_multi_batch_contract_lists() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.data().expect("data config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .build()
        .expect("client should build from live service config");

    let contracts = discover_option_contracts(service, None, "SPY", 180)
        .await
        .expect("should discover a broad SPY contract slice");
    let symbols = contracts
        .into_iter()
        .take(101)
        .map(|contract| contract.symbol)
        .collect::<Vec<_>>();
    assert_eq!(
        symbols.len(),
        101,
        "SPY contract discover should yield >100 symbols"
    );

    let response = client
        .options()
        .snapshots_all(SnapshotsRequest {
            symbols: symbols.clone(),
            feed: Some(preferred_feed()),
            limit: Some(1000),
            page_token: None,
        })
        .await
        .expect("snapshots_all should absorb Alpaca's 100-symbol limit internally");

    assert_eq!(response.snapshots.len(), symbols.len());
}

#[tokio::test]
async fn options_chain_absorbs_brk_b_provider_symbol_rules() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let service = env.data().expect("data config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .build()
        .expect("client should build from live service config");

    let request = ChainRequest {
        underlying_symbol: "BRK.B".to_owned(),
        feed: Some(preferred_feed()),
        r#type: None,
        strike_price_gte: None,
        strike_price_lte: None,
        expiration_date: None,
        expiration_date_gte: None,
        expiration_date_lte: None,
        root_symbol: Some("BRK.B".to_owned()),
        updated_since: None,
        limit: Some(8),
        page_token: None,
    };

    assert_eq!(
        options_underlying_symbol(&request.underlying_symbol),
        "BRKB"
    );

    let chain = client
        .options()
        .chain(request)
        .await
        .expect("BRK.B chain request should succeed through canonical symbol normalization");

    assert!(!chain.snapshots.is_empty());
    assert!(
        alpaca_data::options::ordered_snapshots(&chain.snapshots)
            .iter()
            .all(|(symbol, _)| symbol.starts_with("BRKB")),
        "BRK.B chain contracts should use the BRKB OCC root"
    );
}
