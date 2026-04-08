#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    options::{
        BarsRequest, ChainRequest, ConditionCodesRequest, ContractType, LatestQuotesRequest,
        LatestTradesRequest, OptionsFeed, SnapshotsRequest, TickType, TimeFrame, TradesRequest,
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
    let nearby_contracts =
        discover_option_contracts(&probe, service, Some(&recorder), &contract.underlying_symbol, 8)
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

    let latest_quotes = options
        .latest_quotes(LatestQuotesRequest {
            symbols: vec![contract.symbol.clone()],
            feed: Some(OptionsFeed::Indicative),
        })
        .await
        .expect("latest option quotes should read from real API");
    recorder
        .record_json("alpaca-data-options", "latest-quotes", &latest_quotes)
        .expect("latest quotes sample should record");
    assert!(latest_quotes.quotes.contains_key(&contract.symbol));

    let latest_trades = options
        .latest_trades(LatestTradesRequest {
            symbols: vec![contract.symbol.clone()],
            feed: Some(OptionsFeed::Indicative),
        })
        .await
        .expect("latest option trades should read from real API");
    recorder
        .record_json("alpaca-data-options", "latest-trades", &latest_trades)
        .expect("latest trades sample should record");
    assert!(latest_trades.trades.contains_key(&contract.symbol));

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

    let bars = options
        .bars_all(BarsRequest {
            symbols: vec![contract.symbol.clone()],
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
    let contract_bars = bars
        .bars
        .get(&contract.symbol)
        .expect("bars should include the discovered contract");
    assert!(!contract_bars.is_empty());

    let trades = options
        .trades_all(TradesRequest {
            symbols: vec![contract.symbol.clone()],
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
    let contract_trades = trades
        .trades
        .get(&contract.symbol)
        .expect("trades should include the discovered contract");
    assert!(!contract_trades.is_empty());

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
