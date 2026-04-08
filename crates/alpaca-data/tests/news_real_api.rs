#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    news::{ListRequest, Sort},
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder, full_day_window_from_timestamp};

#[tokio::test]
async fn news_resource_reads_real_api_endpoint() {
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
    let news = client.news();
    let recorder = SampleRecorder::from_live_env(&env);

    let first_page = news
        .list(ListRequest {
            start: None,
            end: None,
            sort: Some(Sort::Desc),
            symbols: Some(vec!["AAPL".to_owned(), "MSFT".to_owned()]),
            limit: Some(1),
            include_content: Some(false),
            exclude_contentless: None,
            page_token: None,
        })
        .await
        .expect("news list should read from real API");
    recorder
        .record_json("alpaca-data-news", "list-page-1", &first_page)
        .expect("news list sample should record");
    assert_eq!(first_page.news.len(), 1);
    assert!(first_page.next_page_token.is_some());
    let window = full_day_window_from_timestamp(&first_page.news[0].created_at)
        .expect("first news item timestamp should produce a full-day window");

    let windowed_first_page = news
        .list(ListRequest {
            start: Some(window.start.clone()),
            end: Some(window.end.clone()),
            sort: Some(Sort::Desc),
            symbols: Some(vec!["AAPL".to_owned(), "MSFT".to_owned()]),
            limit: Some(1),
            include_content: Some(false),
            exclude_contentless: None,
            page_token: None,
        })
        .await
        .expect("windowed news list should read from real API");
    assert!(windowed_first_page.next_page_token.is_some());

    let all_pages = news
        .list_all(ListRequest {
            start: Some(window.start),
            end: Some(window.end),
            sort: Some(Sort::Desc),
            symbols: Some(vec!["AAPL".to_owned(), "MSFT".to_owned()]),
            limit: Some(1),
            include_content: Some(false),
            exclude_contentless: None,
            page_token: None,
        })
        .await
        .expect("news list_all should paginate through real API");
    recorder
        .record_json("alpaca-data-news", "list-all", &all_pages)
        .expect("news list_all sample should record");
    assert!(all_pages.news.len() > 1);
    assert!(all_pages.next_page_token.is_none());
    assert!(
        all_pages
            .news
            .iter()
            .all(|item| !item.headline.trim().is_empty())
    );
    assert!(all_pages.news.iter().any(|item| {
        item.symbols
            .iter()
            .any(|symbol| symbol == "AAPL" || symbol == "MSFT")
    }));
}
