#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{
    Client, Error,
    assets::{UsCorporatesRequest, UsTreasuriesRequest},
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn assets_resource_reads_real_paper_fixed_income_us_corporates() {
    let Some((client, recorder)) = live_client_and_recorder() else {
        return;
    };

    let corporates = match client
        .assets()
        .fixed_income_us_corporates(UsCorporatesRequest {
            bond_status: Some("outstanding".to_owned()),
            tickers: Some(vec!["MSFT".to_owned()]),
            ..UsCorporatesRequest::default()
        })
        .await
    {
        Ok(response) => response,
        Err(error) if is_fixed_income_subscription_gate(&error) => {
            eprintln!("skipping fixed income corporates test: {error}");
            return;
        }
        Err(error) => {
            panic!("fixed income corporates request should succeed against real paper API: {error}")
        }
    };

    recorder
        .record_json(
            "alpaca-trade-assets",
            "fixed-income-us-corporates",
            &corporates,
        )
        .expect("fixed income corporates sample should record");
    assert!(!corporates.us_corporates.is_empty());
    assert!(!corporates.us_corporates[0].ticker.is_empty());
    assert!(!corporates.us_corporates[0].isin.is_empty());
}

#[tokio::test]
async fn assets_resource_reads_real_paper_fixed_income_us_treasuries() {
    let Some((client, recorder)) = live_client_and_recorder() else {
        return;
    };

    let treasuries = match client
        .assets()
        .fixed_income_us_treasuries(UsTreasuriesRequest {
            bond_status: Some("outstanding".to_owned()),
            subtype: Some("note".to_owned()),
            ..UsTreasuriesRequest::default()
        })
        .await
    {
        Ok(response) => response,
        Err(error) if is_fixed_income_subscription_gate(&error) => {
            eprintln!("skipping fixed income treasuries test: {error}");
            return;
        }
        Err(error) => {
            panic!("fixed income treasuries request should succeed against real paper API: {error}")
        }
    };

    recorder
        .record_json(
            "alpaca-trade-assets",
            "fixed-income-us-treasuries",
            &treasuries,
        )
        .expect("fixed income treasuries sample should record");
    assert!(!treasuries.us_treasuries.is_empty());
    assert!(!treasuries.us_treasuries[0].isin.is_empty());
}

fn live_client_and_recorder() -> Option<(Client, SampleRecorder)> {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Trade) {
        eprintln!("skipping real API test: {reason}");
        return None;
    }

    let service = env.trade().expect("trade config should exist");
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .base_url(service.base_url().clone())
        .build()
        .expect("trade client should build from live service config");
    let recorder = SampleRecorder::from_live_env(&env);

    Some((client, recorder))
}

fn is_fixed_income_subscription_gate(error: &Error) -> bool {
    error.meta().is_some_and(|meta| {
        meta.status() == 403
            && meta
                .body_snippet()
                .is_some_and(|body| body.contains("subscription does not permit"))
    })
}
