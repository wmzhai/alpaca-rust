#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_trade::{
    Client,
    calendar::{ListRequest, ListV3Request},
    clock::GetV3Request,
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn calendar_resource_reads_real_paper_v3_calendar_window() {
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
    let clock = client
        .clock()
        .get_v3(GetV3Request {
            markets: Some(vec!["NYSE".to_owned()]),
            ..GetV3Request::default()
        })
        .await
        .expect("v3 clock request should succeed before the v3 calendar query");
    let first_clock = clock
        .clocks
        .first()
        .expect("v3 clock response should include at least one market clock");
    let start_source = if first_clock.is_market_day {
        first_clock.timestamp.as_str()
    } else {
        first_clock.next_market_open.as_str()
    };
    let start = start_source
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .unwrap_or_else(|| start_source[..10].to_owned());

    let calendar = client
        .calendar()
        .list_v3(
            "NYSE",
            ListV3Request {
                start: Some(start.clone()),
                end: Some(start),
                timezone: Some("UTC".to_owned()),
            },
        )
        .await
        .expect("v3 calendar request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-calendar", "list-v3", &calendar)
        .expect("v3 calendar sample should record");

    assert!(!calendar.market.name.is_empty());
    assert!(!calendar.calendar.is_empty());
    assert!(!calendar.calendar[0].date.is_empty());
    assert!(!calendar.calendar[0].core_start.is_empty());
    assert!(!calendar.calendar[0].core_end.is_empty());
}

#[tokio::test]
async fn calendar_resource_reads_real_paper_calendar_window() {
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
    let clock = client
        .clock()
        .get()
        .await
        .expect("clock request should succeed before the calendar query");
    let start = clock
        .timestamp
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .unwrap_or_else(|| clock.timestamp[..10].to_owned());
    let end = clock
        .next_open
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .unwrap_or_else(|| clock.next_open[..10].to_owned());

    let calendar = client
        .calendar()
        .list(ListRequest {
            start: Some(start),
            end: Some(end),
        })
        .await
        .expect("calendar request should succeed against real paper API");
    recorder
        .record_json("alpaca-trade-calendar", "list", &calendar)
        .expect("calendar sample should record");

    assert!(!calendar.is_empty());
    assert!(!calendar[0].date.is_empty());
    assert!(!calendar[0].open.is_empty());
    assert!(!calendar[0].close.is_empty());
    assert!(!calendar[0].session_open.is_empty());
    assert!(!calendar[0].session_close.is_empty());
}
