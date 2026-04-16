#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    options::{
        BarsRequest, ChainRequest, ConditionCodesRequest, ContractType, LatestQuotesRequest,
        LatestTradesRequest, OptionsFeed, SnapshotsRequest, TickType, TimeFrame, TradesRequest,
        underlying_symbol,
    },
};
use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, OptionContractType, SampleRecorder,
    discover_active_option_contract, discover_option_contracts, full_day_window_from_timestamp,
};

#[tokio::test]
async fn options_resource_reads_real_api_endpoints() {
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
    let options = client.options();
    let recorder = SampleRecorder::from_live_env(&env);
    let probe = LiveHttpProbe::new().expect("live probe should build");

    let contract = discover_contract(&probe, service, &recorder).await;
    let nearby_contracts = discover_option_contracts(
        &probe,
        service,
        Some(&recorder),
        &contract.underlying_symbol,
        8,
    )
    .await
    .expect("should discover nearby option contracts");
    let symbols = nearby_contracts
        .iter()
        .take(2)
        .map(|contract| contract.symbol.clone())
        .collect::<Vec<_>>();
    let window = full_day_window_from_timestamp(
        contract
            .reference_timestamp
            .as_deref()
            .expect("active contract should include a reference timestamp"),
    )
    .expect("reference timestamp should expand into a full-day window");

    let latest_symbols = nearby_contracts
        .iter()
        .take(4)
        .map(|contract| contract.symbol.clone())
        .collect::<Vec<_>>();

    let latest_quotes = options
        .latest_quotes(LatestQuotesRequest {
            symbols: latest_symbols.clone(),
            feed: Some(OptionsFeed::Indicative),
        })
        .await
        .expect("latest option quotes should read from real API");
    recorder
        .record_json("alpaca-data-options", "latest-quotes", &latest_quotes)
        .expect("latest quotes sample should record");
    assert!(
        latest_quotes
            .quotes
            .keys()
            .any(|symbol| latest_symbols.contains(symbol)),
        "latest quotes should include at least one requested contract"
    );

    let latest_trades = options
        .latest_trades(LatestTradesRequest {
            symbols: latest_symbols.clone(),
            feed: Some(OptionsFeed::Indicative),
        })
        .await
        .expect("latest option trades should read from real API");
    recorder
        .record_json("alpaca-data-options", "latest-trades", &latest_trades)
        .expect("latest trades sample should record");
    assert!(
        latest_trades
            .trades
            .keys()
            .all(|symbol| latest_symbols.contains(symbol)),
        "latest trades should not include contracts outside the requested batch"
    );

    let snapshots = options
        .snapshots_all(SnapshotsRequest {
            symbols: symbols.clone(),
            feed: Some(OptionsFeed::Indicative),
            limit: Some(1),
            page_token: None,
        })
        .await
        .expect("option snapshots should paginate through real API");
    recorder
        .record_json("alpaca-data-options", "snapshots-all", &snapshots)
        .expect("snapshots sample should record");
    assert!(snapshots.snapshots.contains_key(&contract.symbol));
    assert_eq!(
        alpaca_data::options::ordered_snapshots(&snapshots.snapshots).len(),
        snapshots.snapshots.len()
    );
    assert!(
        alpaca_data::options::ordered_snapshots(&snapshots.snapshots)
            .iter()
            .all(|(_, snapshot)| snapshot.timestamp().is_some()),
        "ordered snapshots should expose a usable timestamp helper"
    );

    let bars = options
        .bars_all(BarsRequest {
            symbols: latest_symbols.clone(),
            timeframe: TimeFrame::min_1(),
            start: Some(window.start.clone()),
            end: Some(window.end.clone()),
            limit: Some(100),
            sort: None,
            page_token: None,
        })
        .await
        .expect("historical option bars should paginate through real API");
    recorder
        .record_json("alpaca-data-options", "bars-all", &bars)
        .expect("bars sample should record");
    assert!(
        bars.bars
            .keys()
            .all(|symbol| latest_symbols.contains(symbol)),
        "historical bars should stay within the requested contracts"
    );

    let trades = options
        .trades_all(TradesRequest {
            symbols: latest_symbols.clone(),
            start: Some(window.start.clone()),
            end: Some(window.end.clone()),
            limit: Some(100),
            sort: None,
            page_token: None,
        })
        .await
        .expect("historical option trades should paginate through real API");
    recorder
        .record_json("alpaca-data-options", "trades-all", &trades)
        .expect("trades sample should record");
    assert!(
        trades
            .trades
            .keys()
            .all(|symbol| latest_symbols.contains(symbol)),
        "historical trades should stay within the requested contracts"
    );

    let chain = options
        .chain_all(ChainRequest {
            underlying_symbol: contract.underlying_symbol.clone(),
            feed: Some(OptionsFeed::Indicative),
            r#type: Some(contract_type_for_request(contract.contract_type)),
            strike_price_gte: None,
            strike_price_lte: None,
            expiration_date: Some(contract.expiration_date.clone()),
            expiration_date_gte: None,
            expiration_date_lte: None,
            root_symbol: None,
            updated_since: None,
            limit: Some(10),
            page_token: None,
        })
        .await
        .expect("option chain snapshots should paginate through real API");
    recorder
        .record_json("alpaca-data-options", "chain-all", &chain)
        .expect("chain sample should record");
    assert!(chain.snapshots.contains_key(&contract.symbol));
    assert_eq!(
        alpaca_data::options::ordered_snapshots(&chain.snapshots).len(),
        chain.snapshots.len()
    );

    let condition_codes = options
        .condition_codes(ConditionCodesRequest {
            ticktype: TickType::Trade,
        })
        .await
        .expect("option condition codes should read from real API");
    assert!(!condition_codes.is_empty());

    let exchange_codes = options
        .exchange_codes()
        .await
        .expect("option exchange codes should read from real API");
    assert!(!exchange_codes.is_empty());
}

#[tokio::test]
async fn options_snapshots_all_absorbs_multi_batch_contract_lists() {
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
    let probe = LiveHttpProbe::new().expect("live probe should build");

    let contracts = discover_option_contracts(&probe, service, None, "SPY", 180)
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
            feed: Some(OptionsFeed::Indicative),
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
        .base_url(service.base_url().clone())
        .build()
        .expect("client should build from live service config");

    let request = ChainRequest {
        underlying_symbol: "BRK.B".to_owned(),
        feed: Some(OptionsFeed::Indicative),
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

    assert_eq!(underlying_symbol(&request.underlying_symbol), "BRKB");

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

async fn discover_contract(
    probe: &LiveHttpProbe,
    service: &live_support::ServiceConfig,
    recorder: &SampleRecorder,
) -> live_support::ObservedOptionContract {
    let candidates = ["AAPL", "SPY", "QQQ"];

    for underlying_symbol in candidates {
        if let Ok(contract) =
            discover_active_option_contract(probe, service, Some(recorder), underlying_symbol, 32)
                .await
            && contract.reference_timestamp.is_some()
        {
            return contract;
        }
    }

    panic!("failed to discover an active option contract for the test universe");
}

fn contract_type_for_request(contract_type: OptionContractType) -> ContractType {
    match contract_type {
        OptionContractType::Call => ContractType::Call,
        OptionContractType::Put => ContractType::Put,
    }
}
