#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{Client, options_contracts::ContractType};
use live_support::{
    AlpacaService, LiveTestEnv, OptionContractType, SampleRecorder, discover_active_option_contract,
};

const UNDERLYING_SYMBOL: &str = "SPY";

#[tokio::test]
#[ignore = "slow live options-contracts endpoint; run explicitly when auditing this resource"]
async fn options_contracts_resource_reads_real_contract_by_symbol_and_id() {
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
    let observed =
        discover_active_option_contract(data_service, Some(&recorder), UNDERLYING_SYMBOL, 8)
            .await
            .expect("a live option contract should be discoverable from real Alpaca data");
    let by_symbol = trade_client
        .options_contracts()
        .get(&observed.symbol)
        .await
        .expect("options contract get by symbol should succeed");
    let by_id = trade_client
        .options_contracts()
        .get(&by_symbol.id)
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
    assert_eq!(
        by_symbol.r#type,
        match observed.contract_type {
            OptionContractType::Call => ContractType::Call,
            OptionContractType::Put => ContractType::Put,
        }
    );
}
