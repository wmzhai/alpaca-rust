#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::sync::Arc;

use alpaca_data::{
    Client,
    crypto::{
        Bar, BarsRequest, LatestBarsRequest, LatestOrderbooksRequest, LatestQuotesRequest,
        LatestTradesRequest, Orderbook, Quote, QuotesRequest, Snapshot, SnapshotsRequest,
        TimeFrame, Trade, TradesRequest, preferred_location,
    },
};
use live_support::{
    LiveRequestObserver, LiveTestEnv, observed_query_value, observed_request_lines,
    unique_observed_requests,
};

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

fn assert_bar_shape(bar: &Bar) {
    assert!(bar.t.is_some(), "bar timestamp should be present");
    assert!(bar.o.is_some(), "bar open should be present");
    assert!(bar.h.is_some(), "bar high should be present");
    assert!(bar.l.is_some(), "bar low should be present");
    assert!(bar.c.is_some(), "bar close should be present");
    assert!(bar.v.is_some(), "bar volume should be present");
    assert!(bar.n.is_some(), "bar trade count should be present");
    assert!(bar.vw.is_some(), "bar VWAP should be present");
}

fn assert_quote_shape(quote: &Quote) {
    assert!(quote.t.is_some(), "quote timestamp should be present");
    assert!(quote.bp.is_some(), "quote bid price should be present");
    assert!(quote.bs.is_some(), "quote bid size should be present");
    assert!(quote.ap.is_some(), "quote ask price should be present");
    assert!(quote.r#as.is_some(), "quote ask size should be present");
}

fn assert_trade_shape(trade: &Trade) {
    assert!(trade.t.is_some(), "trade timestamp should be present");
    assert!(trade.p.is_some(), "trade price should be present");
    assert!(trade.s.is_some(), "trade size should be present");
    assert!(trade.i.is_some(), "trade id should be present");
    assert!(trade.tks.is_some(), "trade taker side should be present");
}

fn assert_orderbook_shape(orderbook: &Orderbook) {
    assert!(
        orderbook.t.is_some(),
        "orderbook timestamp should be present"
    );
}

fn assert_snapshot_shape(snapshot: &Snapshot) {
    assert_trade_shape(
        snapshot
            .latest_trade
            .as_ref()
            .expect("snapshot latestTrade should be present"),
    );
    assert_quote_shape(
        snapshot
            .latest_quote
            .as_ref()
            .expect("snapshot latestQuote should be present"),
    );
    assert_bar_shape(
        snapshot
            .minute_bar
            .as_ref()
            .expect("snapshot minuteBar should be present"),
    );
    assert_bar_shape(
        snapshot
            .daily_bar
            .as_ref()
            .expect("snapshot dailyBar should be present"),
    );
    assert_bar_shape(
        snapshot
            .prev_daily_bar
            .as_ref()
            .expect("snapshot prevDailyBar should be present"),
    );
}

#[tokio::test]
async fn crypto_latest_bars_use_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .latest_bars(LatestBarsRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned(), "ETH/USD".to_owned()],
        })
        .await
        .expect("latest crypto bars should read from the real Data API");

    let btc = response
        .bars
        .get("BTC/USD")
        .expect("canonical latest bars response should contain BTC/USD");
    let eth = response
        .bars
        .get("ETH/USD")
        .expect("canonical latest bars response should contain ETH/USD");
    assert_bar_shape(btc);
    assert_bar_shape(eth);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("crypto.latest_bars"));
    assert!(request.url.contains("/v1beta3/crypto/us/latest/bars"));
    assert_eq!(
        observed_query_value(&request, "symbols").as_deref(),
        Some("BTC/USD,ETH/USD")
    );

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    let request_id = meta
        .request_id()
        .expect("real Data API response should include x-request-id");
    eprintln!(
        "real_api operation={} request={} {} status={} request_id={} shape=bars{{BTC/USD,ETH/USD}}(t,o,h,l,c,v,n,vw)",
        request.operation.as_deref().unwrap_or("unknown"),
        request.method,
        request.url,
        meta.status(),
        request_id
    );
}

#[tokio::test]
async fn crypto_latest_quotes_use_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .latest_quotes(LatestQuotesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
        })
        .await
        .expect("latest crypto quotes should read from the real Data API");

    let quote = response
        .quotes
        .get("BTC/USD")
        .expect("canonical latest quotes response should contain BTC/USD");
    assert_quote_shape(quote);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("crypto.latest_quotes"));
    assert!(request.url.contains("/v1beta3/crypto/us/latest/quotes"));
    assert_eq!(
        observed_query_value(&request, "symbols").as_deref(),
        Some("BTC/USD")
    );

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    assert!(meta.request_id().is_some());
}

#[tokio::test]
async fn crypto_latest_trades_use_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .latest_trades(LatestTradesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
        })
        .await
        .expect("latest crypto trades should read from the real Data API");

    let trade = response
        .trades
        .get("BTC/USD")
        .expect("canonical latest trades response should contain BTC/USD");
    assert_trade_shape(trade);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("crypto.latest_trades"));
    assert!(request.url.contains("/v1beta3/crypto/us/latest/trades"));
    assert_eq!(
        observed_query_value(&request, "symbols").as_deref(),
        Some("BTC/USD")
    );

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    assert!(meta.request_id().is_some());
}

#[tokio::test]
async fn crypto_latest_orderbooks_use_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .latest_orderbooks(LatestOrderbooksRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
        })
        .await
        .expect("latest crypto orderbooks should read from the real Data API");

    let orderbook = response
        .orderbooks
        .get("BTC/USD")
        .expect("canonical latest orderbooks response should contain BTC/USD");
    assert_orderbook_shape(orderbook);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(
        request.operation.as_deref(),
        Some("crypto.latest_orderbooks")
    );
    assert!(request.url.contains("/v1beta3/crypto/us/latest/orderbooks"));
    assert_eq!(
        observed_query_value(&request, "symbols").as_deref(),
        Some("BTC/USD")
    );

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    assert!(meta.request_id().is_some());
}

#[tokio::test]
async fn crypto_snapshots_use_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .snapshots(SnapshotsRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
        })
        .await
        .expect("crypto snapshots should read from the real Data API");

    let snapshot = response
        .snapshots
        .get("BTC/USD")
        .expect("canonical snapshots response should contain BTC/USD");
    assert_snapshot_shape(snapshot);
    assert!(
        snapshot.price().is_some(),
        "snapshot price helper should resolve"
    );

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("crypto.snapshots"));
    assert!(request.url.contains("/v1beta3/crypto/us/snapshots"));
    assert_eq!(
        observed_query_value(&request, "symbols").as_deref(),
        Some("BTC/USD")
    );

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    assert!(meta.request_id().is_some());
}

#[tokio::test]
async fn crypto_bars_use_real_api_endpoint_and_paginate() {
    let (client, observer) = real_data_client();
    let response = client
        .crypto()
        .bars_all(BarsRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
            timeframe: TimeFrame::min_1(),
            start: Some("2026-08-20T15:00:00Z".to_owned()),
            end: Some("2026-08-20T15:04:00Z".to_owned()),
            limit: Some(2),
            sort: Some(alpaca_data::crypto::Sort::Asc),
            page_token: None,
        })
        .await
        .expect("crypto bars should paginate through the real Data API");

    let bars = response
        .bars
        .get("BTC/USD")
        .expect("canonical bars response should contain BTC/USD");
    assert!(bars.len() > 2);
    bars.iter().for_each(assert_bar_shape);
    assert!(response.next_page_token.is_none());

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert!(
        responses.len() > 1,
        "limit=2 should exercise real pagination"
    );
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("crypto.bars")
            && request.url.contains("/v1beta3/crypto/us/bars")
            && observed_query_value(request, "symbols").as_deref() == Some("BTC/USD")
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
    eprintln!(
        "real_api operation=crypto.bars pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=bars{{BTC/USD}}[]+next_page_token bars={}",
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
        bars.len()
    );
}

#[tokio::test]
async fn crypto_quotes_use_real_api_endpoint_and_paginate() {
    let (client, observer) = real_data_client();
    let first = client
        .crypto()
        .quotes(QuotesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
            start: Some("2026-08-20T15:00:00Z".to_owned()),
            end: Some("2026-08-20T15:01:00Z".to_owned()),
            limit: Some(2),
            sort: Some(alpaca_data::crypto::Sort::Asc),
            page_token: None,
        })
        .await
        .expect("crypto quotes should read from the real Data API");

    let quotes = first
        .quotes
        .get("BTC/USD")
        .expect("canonical quotes response should contain BTC/USD");
    assert_eq!(quotes.len(), 2);
    quotes.iter().for_each(assert_quote_shape);
    let page_token = first
        .next_page_token
        .clone()
        .expect("limit=2 should return a next_page_token");

    let second = client
        .crypto()
        .quotes(QuotesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
            start: Some("2026-08-20T15:00:00Z".to_owned()),
            end: Some("2026-08-20T15:01:00Z".to_owned()),
            limit: Some(2),
            sort: Some(alpaca_data::crypto::Sort::Asc),
            page_token: Some(page_token),
        })
        .await
        .expect("crypto quotes should continue from next_page_token");
    let next_quotes = second
        .quotes
        .get("BTC/USD")
        .expect("paginated quotes response should contain BTC/USD");
    assert!(!next_quotes.is_empty());
    next_quotes.iter().for_each(assert_quote_shape);

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("crypto.quotes")
            && request.url.contains("/v1beta3/crypto/us/quotes")
            && observed_query_value(request, "symbols").as_deref() == Some("BTC/USD")
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
}

#[tokio::test]
async fn crypto_trades_use_real_api_endpoint_and_paginate() {
    let (client, observer) = real_data_client();
    let first = client
        .crypto()
        .trades(TradesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
            start: Some("2026-08-20T15:00:00Z".to_owned()),
            end: Some("2026-08-20T15:10:00Z".to_owned()),
            limit: Some(2),
            sort: Some(alpaca_data::crypto::Sort::Asc),
            page_token: None,
        })
        .await
        .expect("crypto trades should read from the real Data API");

    let trades = first
        .trades
        .get("BTC/USD")
        .expect("canonical trades response should contain BTC/USD");
    assert_eq!(trades.len(), 2);
    trades.iter().for_each(assert_trade_shape);
    let page_token = first
        .next_page_token
        .clone()
        .expect("limit=2 should return a next_page_token");

    let second = client
        .crypto()
        .trades(TradesRequest {
            location: preferred_location(),
            symbols: vec!["BTC/USD".to_owned()],
            start: Some("2026-08-20T15:00:00Z".to_owned()),
            end: Some("2026-08-20T15:10:00Z".to_owned()),
            limit: Some(2),
            sort: Some(alpaca_data::crypto::Sort::Asc),
            page_token: Some(page_token),
        })
        .await
        .expect("crypto trades should continue from next_page_token");
    let next_trades = second
        .trades
        .get("BTC/USD")
        .expect("paginated trades response should contain BTC/USD");
    assert!(!next_trades.is_empty());
    next_trades.iter().for_each(assert_trade_shape);

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert_eq!(responses.len(), 2);
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("crypto.trades")
            && request.url.contains("/v1beta3/crypto/us/trades")
            && observed_query_value(request, "symbols").as_deref() == Some("BTC/USD")
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
}
