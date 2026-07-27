use alpaca_core::BaseUrl;
use alpaca_data::Client;
use alpaca_data::cache::CachedClient;
use alpaca_data::stocks::{SnapshotsRequest, preferred_feed};
use rust_decimal::Decimal;
use serde_json::json;

const API_KEY: &str = "runtime-stock-snapshot-key";
const SECRET_KEY: &str = "runtime-stock-snapshot-secret";

#[tokio::test]
async fn runtime_stock_snapshot_uses_data_builder_override_and_cached_single_endpoint() {
    let server = alpaca_mock::spawn_test_server().await;
    reqwest::Client::new()
        .post(format!("{}/admin/market-data/stocks/gld", server.base_url))
        .json(&json!({ "price": "187.34" }))
        .send()
        .await
        .expect("runtime stock price request should complete")
        .error_for_status()
        .expect("runtime stock price should be accepted");

    let raw = Client::builder()
        .api_key(API_KEY)
        .secret_key(SECRET_KEY)
        .base_url_str(&server.base_url)
        .expect("mock Data API base URL should be accepted")
        .build()
        .expect("Data API client should build");
    assert_eq!(raw.base_url().as_str(), server.base_url);

    let snapshots = raw
        .stocks()
        .snapshots(SnapshotsRequest {
            symbols: vec!["GLD".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("single stock snapshot should load through the mock Data API");
    assert_runtime_price(
        snapshots
            .get("GLD")
            .expect("single snapshot response should contain GLD"),
    );

    let base_url = BaseUrl::new(&server.base_url).expect("mock base URL should be valid");
    let cached = CachedClient::new(
        Client::builder()
            .api_key(API_KEY)
            .secret_key(SECRET_KEY)
            .base_url(base_url)
            .build()
            .expect("Data API client should build from BaseUrl"),
    );
    let snapshot = cached
        .stock("gld")
        .await
        .expect("CachedClient should load the controlled stock snapshot");
    assert_runtime_price(&snapshot);
}

fn assert_runtime_price(snapshot: &alpaca_data::stocks::Snapshot) {
    let expected = Decimal::new(18_734, 2);
    let trade = snapshot
        .latest_trade
        .as_ref()
        .expect("snapshot should contain latestTrade");
    let quote = snapshot
        .latest_quote
        .as_ref()
        .expect("snapshot should contain latestQuote");
    assert_eq!(trade.p, Some(expected));
    assert_eq!(quote.bp, Some(expected));
    assert_eq!(quote.ap, Some(expected));
}
