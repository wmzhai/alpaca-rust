#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::sync::Arc;

use alpaca_data::{
    Client,
    corporate_actions::{CorporateActionType, ListRequest, Region, Sort},
};
use live_support::{
    LiveRequestObserver, LiveTestEnv, observed_query_value, observed_request_lines,
    unique_observed_requests,
};
use serde_json::Value;

fn real_data_client() -> (Client, Arc<LiveRequestObserver>) {
    let env = LiveTestEnv::load().expect("live test environment should load");
    let service = env
        .data()
        .expect("Paper/Data credentials must be present for real API tests");
    let observer = Arc::new(LiveRequestObserver::default());
    let client = Client::builder()
        .credentials(service.credentials().clone())
        .observer(observer.clone())
        .build()
        .expect("client should build from Paper/Data credentials");
    (client, observer)
}

fn all_action_types() -> Vec<CorporateActionType> {
    vec![
        CorporateActionType::ForwardSplit,
        CorporateActionType::ReverseSplit,
        CorporateActionType::UnitSplit,
        CorporateActionType::StockDividend,
        CorporateActionType::CashDividend,
        CorporateActionType::SpinOff,
        CorporateActionType::CashMerger,
        CorporateActionType::StockMerger,
        CorporateActionType::StockAndCashMerger,
        CorporateActionType::Redemption,
        CorporateActionType::NameChange,
        CorporateActionType::WorthlessRemoval,
        CorporateActionType::RightsDistribution,
        CorporateActionType::PartialCall,
        CorporateActionType::Reorganization,
    ]
}

fn count_non_null_fields(value: &Value, predicate: &impl Fn(&str) -> bool) -> usize {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| count_non_null_fields(value, predicate))
            .sum(),
        Value::Object(fields) => fields
            .iter()
            .map(|(name, value)| {
                usize::from(predicate(name) && !value.is_null())
                    + count_non_null_fields(value, predicate)
            })
            .sum(),
        _ => 0,
    }
}

#[tokio::test]
async fn corporate_actions_regions_and_types_use_real_api_contract() {
    let (client, observer) = real_data_client();
    let expected_types = all_action_types()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let us = client
        .corporate_actions()
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: Some(all_action_types()),
            region: Some(Region::Us),
            start: Some("2026-07-01".to_owned()),
            end: Some("2026-07-10".to_owned()),
            ids: None,
            limit: Some(1000),
            sort: Some(Sort::Asc),
            page_token: None,
        })
        .await
        .expect("all corporate action types should read from the real US Data API");
    assert!(!us.corporate_actions.cash_dividends.is_empty());
    assert!(!us.corporate_actions.reorganizations.is_empty());
    let cash_dividends_with_sub_type = us
        .corporate_actions
        .cash_dividends
        .iter()
        .filter(|action| action.sub_type.is_some())
        .count();
    assert!(
        cash_dividends_with_sub_type > 0,
        "real cash dividends should exercise typed sub_type"
    );

    let non_us = client
        .corporate_actions()
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: Some(all_action_types()),
            region: Some(Region::NonUs),
            start: Some("2026-07-01".to_owned()),
            end: Some("2026-07-10".to_owned()),
            ids: None,
            limit: Some(1000),
            sort: Some(Sort::Asc),
            page_token: None,
        })
        .await
        .expect("non-US corporate actions should read from the real Data API");
    let serialized_non_us_actions = serde_json::to_value(&non_us.corporate_actions)
        .expect("typed non-US corporate actions should serialize for field-presence verification");
    let non_us_currency_values =
        count_non_null_fields(&serialized_non_us_actions, &|name| name == "currency");
    let non_us_isin_values =
        count_non_null_fields(&serialized_non_us_actions, &|name| name.ends_with("isin"));
    assert!(
        non_us_currency_values > 0,
        "real non-US actions should exercise typed currency fields"
    );
    assert!(
        non_us_isin_values > 0,
        "real non-US actions should exercise typed ISIN fields"
    );

    let all = client
        .corporate_actions()
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: Some(vec![CorporateActionType::PartialCall]),
            region: Some(Region::All),
            start: Some("2026-03-31".to_owned()),
            end: Some("2026-07-08".to_owned()),
            ids: None,
            limit: Some(100),
            sort: Some(Sort::Asc),
            page_token: None,
        })
        .await
        .expect("all-region partial calls should read from the real Data API");
    assert!(!all.corporate_actions.partial_calls.is_empty());
    assert!(
        all.corporate_actions
            .partial_calls
            .iter()
            .all(|action| !action.id.is_empty()
                && !action.symbol.is_empty()
                && !action.process_date.is_empty())
    );
    let partial_calls_with_lottery_type = all
        .corporate_actions
        .partial_calls
        .iter()
        .filter(|action| action.lottery_type.is_some())
        .count();
    assert!(
        partial_calls_with_lottery_type > 0,
        "real partial calls should exercise typed lottery_type"
    );

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert_eq!(requests.len(), 3);
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("corporate_actions.list")
            && request.url.contains("/v1/corporate-actions")
    }));
    assert_eq!(
        observed_query_value(&requests[0], "types").as_deref(),
        Some(expected_types.as_str())
    );
    assert_eq!(
        observed_query_value(&requests[1], "types").as_deref(),
        Some(expected_types.as_str())
    );
    assert_eq!(
        observed_query_value(&requests[0], "region").as_deref(),
        Some("us")
    );
    assert_eq!(
        observed_query_value(&requests[1], "region").as_deref(),
        Some("non_us")
    );
    assert_eq!(
        observed_query_value(&requests[2], "region").as_deref(),
        Some("all")
    );
    assert!(
        responses
            .iter()
            .all(|meta| meta.status() == 200 && meta.request_id().is_some())
    );
    eprintln!(
        "real_api operation=corporate_actions.list attempts={} retries={:?} requests={:?} regions={:?} statuses={:?} request_ids={:?} shape=typed_15_action_arrays cash_dividends={} reorganizations={} partial_calls={} non_us_currency_values={} non_us_isin_values={} cash_dividend_sub_type={} partial_call_lottery_type={}",
        attempts.len(),
        retries
            .iter()
            .map(|retry| retry.status.map(|status| status.as_u16()))
            .collect::<Vec<_>>(),
        observed_request_lines(&attempts),
        requests
            .iter()
            .filter_map(|request| observed_query_value(request, "region"))
            .collect::<Vec<_>>(),
        responses
            .iter()
            .map(|meta| meta.status())
            .collect::<Vec<_>>(),
        responses
            .iter()
            .filter_map(|meta| meta.request_id())
            .collect::<Vec<_>>(),
        us.corporate_actions.cash_dividends.len(),
        us.corporate_actions.reorganizations.len(),
        all.corporate_actions.partial_calls.len(),
        non_us_currency_values,
        non_us_isin_values,
        cash_dividends_with_sub_type,
        partial_calls_with_lottery_type
    );
}

#[tokio::test]
async fn corporate_action_reorganizations_paginate_real_api() {
    let (client, observer) = real_data_client();
    let response = client
        .corporate_actions()
        .list_all(ListRequest {
            symbols: None,
            cusips: None,
            types: Some(vec![CorporateActionType::Reorganization]),
            region: Some(Region::Us),
            start: Some("2026-05-08".to_owned()),
            end: Some("2026-05-08".to_owned()),
            ids: None,
            limit: Some(1),
            sort: Some(Sort::Asc),
            page_token: None,
        })
        .await
        .expect("real reorganizations should paginate through the Data API");

    assert!(response.corporate_actions.reorganizations.len() > 1);
    assert!(response.next_page_token.is_none());
    assert!(
        response
            .corporate_actions
            .reorganizations
            .iter()
            .any(|action| !action.stock_movements.is_empty())
    );
    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert!(responses.len() > 1);
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("corporate_actions.list")
            && request.url.contains("/v1/corporate-actions")
            && observed_query_value(request, "types").as_deref() == Some("reorganization")
            && observed_query_value(request, "region").as_deref() == Some("us")
    }));
    assert!(
        responses
            .iter()
            .all(|meta| meta.status() == 200 && meta.request_id().is_some())
    );
    eprintln!(
        "real_api operation=corporate_actions.list_all pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=reorganizations[]+stock_movements[]+next_page_token reorganizations={}",
        responses.len(),
        attempts.len(),
        retries
            .iter()
            .map(|retry| retry.status.map(|status| status.as_u16()))
            .collect::<Vec<_>>(),
        observed_request_lines(&attempts),
        responses
            .iter()
            .map(|meta| meta.status())
            .collect::<Vec<_>>(),
        responses
            .iter()
            .filter_map(|meta| meta.request_id())
            .collect::<Vec<_>>(),
        response.corporate_actions.reorganizations.len()
    );
}
