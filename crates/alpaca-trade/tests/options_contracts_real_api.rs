#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{
    Client,
    options_contracts::{ContractStatus, ContractType, ListRequest},
};
use live_support::{
    AlpacaService, LiveHttpProbe, LiveTestEnv, OptionContractType, SampleRecorder,
    discover_active_option_contract,
};

const UNDERLYING_SYMBOL: &str = "SPY";

#[tokio::test]
async fn options_contracts_resource_reads_real_contract_list_and_single_contract() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return;
    }
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping real API test: {reason}");
        return;
    }

    let trade_service = env.trade().expect("trade config should exist");
    let data_service = env.data().expect("data config should exist");
    let trade_client = Client::builder()
        .credentials(trade_service.credentials().clone())
        .base_url(trade_service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);
    let probe = LiveHttpProbe::new().expect("live probe should build");
    let observed = discover_active_option_contract(
        &probe,
        data_service,
        Some(&recorder),
        UNDERLYING_SYMBOL,
        200,
    )
    .await
    .expect("a live option contract should be discoverable from real Alpaca data");
    let contract_type = match observed.contract_type {
        OptionContractType::Call => ContractType::Call,
        OptionContractType::Put => ContractType::Put,
    };

    let listed = trade_client
        .options_contracts()
        .list(ListRequest {
            underlying_symbols: Some(vec![observed.underlying_symbol.clone()]),
            show_deliverables: Some(true),
            status: Some(ContractStatus::Active),
            expiration_date: Some(observed.expiration_date.clone()),
            r#type: Some(contract_type.clone()),
            strike_price_gte: Some(observed.strike_price),
            strike_price_lte: Some(observed.strike_price),
            limit: Some(100),
            ..ListRequest::default()
        })
        .await
        .expect("options contracts list request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-options-contracts", "list", &listed)
        .expect("options contracts list sample should record");

    let listed_all = trade_client
        .options_contracts()
        .list_all(ListRequest {
            underlying_symbols: Some(vec![observed.underlying_symbol.clone()]),
            show_deliverables: Some(true),
            status: Some(ContractStatus::Active),
            expiration_date: Some(observed.expiration_date.clone()),
            r#type: Some(contract_type.clone()),
            limit: Some(1),
            ..ListRequest::default()
        })
        .await
        .expect("options contracts list_all request should paginate through real paper API");
    recorder
        .record_json("alpaca-trade-options-contracts", "list-all", &listed_all)
        .expect("options contracts list_all sample should record");
    assert!(listed_all.next_page_token.is_none());
    assert!(
        listed_all
            .option_contracts
            .iter()
            .any(|contract| contract.symbol == observed.symbol)
    );
    assert!(listed_all.option_contracts.len() >= listed.option_contracts.len());

    let matched = listed
        .option_contracts
        .iter()
        .find(|contract| contract.symbol == observed.symbol)
        .expect("listed contracts should contain the discovered live contract");
    let by_symbol = trade_client
        .options_contracts()
        .get(&observed.symbol)
        .await
        .expect("options contract get by symbol should succeed");
    let by_id = trade_client
        .options_contracts()
        .get(&matched.id)
        .await
        .expect("options contract get by id should succeed");
    recorder
        .record_json("alpaca-trade-options-contracts", "get", &by_symbol)
        .expect("options contract get sample should record");

    assert_eq!(by_symbol.id, by_id.id);
    assert_eq!(by_symbol.symbol, observed.symbol);
    assert_eq!(by_symbol.underlying_symbol, observed.underlying_symbol);
    assert_eq!(by_symbol.expiration_date, observed.expiration_date);
    assert_eq!(by_symbol.strike_price, observed.strike_price);
}
