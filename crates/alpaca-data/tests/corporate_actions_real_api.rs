#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    corporate_actions::{CorporateActionType, ListRequest, Sort},
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn corporate_actions_resource_reads_real_api_endpoint() {
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
    let corporate_actions = client.corporate_actions();
    let recorder = SampleRecorder::from_live_env(&env);

    let first_page = corporate_actions
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: Some(vec![CorporateActionType::CashDividend]),
            start: Some("2024-07-31".to_owned()),
            end: Some("2024-08-01".to_owned()),
            ids: None,
            limit: Some(2),
            sort: Some(Sort::Desc),
            page_token: None,
        })
        .await
        .expect("cash dividend page should read from real API");
    recorder
        .record_json(
            "alpaca-data-corporate-actions",
            "cash-dividends-page-1",
            &first_page,
        )
        .expect("cash dividend sample should record");
    assert!(first_page.corporate_actions.cash_dividends.len() >= 2);
    assert!(first_page.next_page_token.is_some());

    let dividend_id = first_page.corporate_actions.cash_dividends[0].id.clone();
    let dividend_symbol = first_page.corporate_actions.cash_dividends[0]
        .symbol
        .clone();
    let paginated_symbols = first_page
        .corporate_actions
        .cash_dividends
        .iter()
        .take(2)
        .map(|action| action.symbol.clone())
        .collect::<Vec<_>>();

    let all_dividends = corporate_actions
        .list_all(ListRequest {
            symbols: Some(paginated_symbols.clone()),
            cusips: None,
            types: Some(vec![CorporateActionType::CashDividend]),
            start: Some("2024-07-31".to_owned()),
            end: Some("2024-08-01".to_owned()),
            ids: None,
            limit: Some(1),
            sort: Some(Sort::Desc),
            page_token: None,
        })
        .await
        .expect("cash dividend list_all should paginate through real API");
    recorder
        .record_json(
            "alpaca-data-corporate-actions",
            "cash-dividends-all",
            &all_dividends,
        )
        .expect("cash dividend aggregate sample should record");
    assert!(all_dividends.corporate_actions.cash_dividends.len() > 1);
    assert!(all_dividends.next_page_token.is_none());
    assert!(
        all_dividends
            .corporate_actions
            .cash_dividends
            .iter()
            .all(|action| paginated_symbols
                .iter()
                .any(|symbol| symbol == &action.symbol))
    );

    let by_id = corporate_actions
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: None,
            start: None,
            end: None,
            ids: Some(vec![dividend_id.clone()]),
            limit: Some(1),
            sort: None,
            page_token: None,
        })
        .await
        .expect("ids filter should read from real API");
    recorder
        .record_json("alpaca-data-corporate-actions", "by-id", &by_id)
        .expect("id-filter sample should record");
    assert!(
        by_id
            .corporate_actions
            .cash_dividends
            .iter()
            .any(|action| action.id == dividend_id)
    );

    let broad_page = corporate_actions
        .list(ListRequest {
            symbols: None,
            cusips: None,
            types: None,
            start: Some("2024-08-01".to_owned()),
            end: Some("2024-08-20".to_owned()),
            ids: None,
            limit: Some(2),
            sort: Some(Sort::Desc),
            page_token: None,
        })
        .await
        .expect("broad corporate actions page should read from real API");
    recorder
        .record_json("alpaca-data-corporate-actions", "broad-page", &broad_page)
        .expect("broad page sample should record");
    assert!(broad_page.next_page_token.is_some());
    assert!(
        !broad_page.corporate_actions.name_changes.is_empty()
            || !broad_page.corporate_actions.cash_dividends.is_empty()
            || !broad_page.corporate_actions.forward_splits.is_empty()
    );

    let by_symbol = corporate_actions
        .list(ListRequest {
            symbols: Some(vec![dividend_symbol.clone()]),
            cusips: None,
            types: Some(vec![CorporateActionType::CashDividend]),
            start: Some("2024-07-31".to_owned()),
            end: Some("2024-08-01".to_owned()),
            ids: None,
            limit: Some(2),
            sort: Some(Sort::Desc),
            page_token: None,
        })
        .await
        .expect("symbol-filtered corporate actions should read from real API");
    assert!(
        by_symbol
            .corporate_actions
            .cash_dividends
            .iter()
            .all(|action| action.symbol == dividend_symbol)
    );
}
