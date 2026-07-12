#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use std::sync::Arc;

use alpaca_data::{
    Client,
    stocks::{
        Auction, AuctionsRequest, Bar, BarsRequest, ConditionCodesRequest, LatestBarsRequest,
        LatestQuotesRequest, LatestTradesRequest, Quote, QuotesRequest, Snapshot, SnapshotsRequest,
        Tape, TickType, TimeFrame, Trade, TradesRequest, display_stock_symbol, ordered_snapshots,
        preferred_feed,
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
    assert!(quote.ax.is_some(), "quote ask exchange should be present");
    assert!(quote.ap.is_some(), "quote ask price should be present");
    assert!(quote.r#as.is_some(), "quote ask size should be present");
    assert!(quote.bx.is_some(), "quote bid exchange should be present");
    assert!(quote.bp.is_some(), "quote bid price should be present");
    assert!(quote.bs.is_some(), "quote bid size should be present");
    assert!(quote.c.is_some(), "quote conditions should be present");
    assert!(quote.z.is_some(), "quote tape should be present");
}

fn assert_trade_shape(trade: &Trade) {
    assert!(trade.t.is_some(), "trade timestamp should be present");
    assert!(trade.x.is_some(), "trade exchange should be present");
    assert!(trade.p.is_some(), "trade price should be present");
    assert!(trade.s.is_some(), "trade size should be present");
    assert!(trade.i.is_some(), "trade id should be present");
    assert!(trade.c.is_some(), "trade conditions should be present");
    assert!(trade.z.is_some(), "trade tape should be present");
}

fn assert_auction_shape(auction: &Auction) {
    assert!(auction.t.is_some(), "auction timestamp should be present");
    assert!(auction.x.is_some(), "auction exchange should be present");
    assert!(auction.p.is_some(), "auction price should be present");
    assert!(auction.s.is_some(), "auction size should be present");
    assert!(auction.c.is_some(), "auction condition should be present");
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
async fn stock_latest_bar_single_uses_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .latest_bars(LatestBarsRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("single-symbol latest bar should read from the real Data API");

    let bar = response
        .bars
        .get("AAPL")
        .expect("canonical latest bars response should contain AAPL");
    assert_bar_shape(bar);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("stocks.latest_bar"));
    assert!(request.url.contains("/v2/stocks/AAPL/bars/latest"));
    assert!(observed_query_value(&request, "symbols").is_none());

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    let request_id = meta
        .request_id()
        .expect("real Data API response should include x-request-id");
    eprintln!(
        "real_api operation={} request={} {} status={} request_id={} shape=symbol+bar(t,o,h,l,c,v,n,vw)",
        request.operation.as_deref().unwrap_or("unknown"),
        request.method,
        request.url,
        meta.status(),
        request_id
    );
}

#[tokio::test]
async fn stock_bar_single_uses_real_api_endpoint_and_paginates() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .bars_all(BarsRequest {
            symbols: vec!["AAPL".to_owned()],
            timeframe: TimeFrame::day_1(),
            start: Some("2026-07-01".to_owned()),
            end: Some("2026-07-10".to_owned()),
            limit: Some(2),
            adjustment: None,
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            sort: Some(alpaca_data::stocks::Sort::Asc),
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .expect("single-symbol bars should paginate through the real Data API");

    let bars = response
        .bars
        .get("AAPL")
        .expect("canonical bars response should contain AAPL");
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
        request.operation.as_deref() == Some("stocks.bar_single")
            && request.url.contains("/v2/stocks/AAPL/bars")
            && observed_query_value(request, "symbols").is_none()
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
    eprintln!(
        "real_api operation=stocks.bar_single pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=symbol+bars[]+next_page_token bars={}",
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
async fn stock_latest_quote_single_uses_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .latest_quotes(LatestQuotesRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("single-symbol latest quote should read from the real Data API");

    let quote = response
        .quotes
        .get("AAPL")
        .expect("canonical latest quotes response should contain AAPL");
    assert_quote_shape(quote);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("stocks.latest_quote"));
    assert!(request.url.contains("/v2/stocks/AAPL/quotes/latest"));
    assert!(observed_query_value(&request, "symbols").is_none());

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    let request_id = meta
        .request_id()
        .expect("real Data API response should include x-request-id");
    eprintln!(
        "real_api operation={} request={} {} status={} request_id={} shape=symbol+currency+quote(t,ax,ap,as,bx,bp,bs,c,z)",
        request.operation.as_deref().unwrap_or("unknown"),
        request.method,
        request.url,
        meta.status(),
        request_id
    );
}

#[tokio::test]
async fn stock_quote_single_uses_real_api_endpoint_and_paginates() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .quotes_all(QuotesRequest {
            symbols: vec!["AAPL".to_owned()],
            start: Some("2026-07-10T13:30:00Z".to_owned()),
            end: Some("2026-07-10T13:30:00.14835285Z".to_owned()),
            limit: Some(2),
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            sort: Some(alpaca_data::stocks::Sort::Asc),
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .expect("single-symbol quotes should paginate through the real Data API");

    let quotes = response
        .quotes
        .get("AAPL")
        .expect("canonical quotes response should contain AAPL");
    assert!(quotes.len() > 2);
    quotes.iter().for_each(assert_quote_shape);
    assert!(response.next_page_token.is_none());

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert!(
        responses.len() > 1,
        "limit=2 should exercise real quote pagination"
    );
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("stocks.quote_single")
            && request.url.contains("/v2/stocks/AAPL/quotes")
            && observed_query_value(request, "symbols").is_none()
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
    eprintln!(
        "real_api operation=stocks.quote_single pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=symbol+quotes[]+next_page_token quotes={}",
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
        quotes.len()
    );
}

#[tokio::test]
async fn stock_latest_trade_single_uses_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .latest_trades(LatestTradesRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("single-symbol latest trade should read from the real Data API");

    let trade = response
        .trades
        .get("AAPL")
        .expect("canonical latest trades response should contain AAPL");
    assert_trade_shape(trade);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("stocks.latest_trade"));
    assert!(request.url.contains("/v2/stocks/AAPL/trades/latest"));
    assert!(observed_query_value(&request, "symbols").is_none());

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    let request_id = meta
        .request_id()
        .expect("real Data API response should include x-request-id");
    eprintln!(
        "real_api operation={} request={} {} status={} request_id={} shape=symbol+currency+trade(t,x,p,s,i,c,z)",
        request.operation.as_deref().unwrap_or("unknown"),
        request.method,
        request.url,
        meta.status(),
        request_id
    );
}

#[tokio::test]
async fn stock_trade_single_uses_real_api_endpoint_and_paginates() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .trades_all(TradesRequest {
            symbols: vec!["AAPL".to_owned()],
            start: Some("2026-07-10T13:30:00Z".to_owned()),
            end: Some("2026-07-10T13:30:00.117509278Z".to_owned()),
            limit: Some(2),
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            sort: Some(alpaca_data::stocks::Sort::Asc),
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .expect("single-symbol trades should paginate through the real Data API");

    let trades = response
        .trades
        .get("AAPL")
        .expect("canonical trades response should contain AAPL");
    assert!(trades.len() > 2);
    trades.iter().for_each(assert_trade_shape);
    assert!(response.next_page_token.is_none());

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert!(
        responses.len() > 1,
        "limit=2 should exercise real trade pagination"
    );
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("stocks.trade_single")
            && request.url.contains("/v2/stocks/AAPL/trades")
            && observed_query_value(request, "symbols").is_none()
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
    eprintln!(
        "real_api operation=stocks.trade_single pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=symbol+trades[]+next_page_token trades={}",
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
        trades.len()
    );
}

#[tokio::test]
async fn stock_snapshot_single_uses_real_api_endpoint() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .snapshots(SnapshotsRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(alpaca_data::stocks::DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("single-symbol snapshot should read from the real Data API");

    let snapshot = response
        .get("AAPL")
        .expect("canonical snapshots response should contain AAPL");
    assert_snapshot_shape(snapshot);

    let request = observer
        .last_request()
        .expect("real Data API request should be observed");
    assert_eq!(request.operation.as_deref(), Some("stocks.snapshot"));
    assert!(request.url.contains("/v2/stocks/AAPL/snapshot"));
    assert!(observed_query_value(&request, "symbols").is_none());

    let meta = observer
        .last_response()
        .expect("real Data API response should be observed");
    assert_eq!(meta.status(), 200);
    let request_id = meta
        .request_id()
        .expect("real Data API response should include x-request-id");
    eprintln!(
        "real_api operation={} request={} {} status={} request_id={} shape=symbol+latestTrade+latestQuote+minuteBar+dailyBar+prevDailyBar",
        request.operation.as_deref().unwrap_or("unknown"),
        request.method,
        request.url,
        meta.status(),
        request_id
    );
}

#[tokio::test]
async fn stock_auction_single_uses_real_api_endpoint_and_paginates() {
    let (client, observer) = real_data_client();
    let response = client
        .stocks()
        .auctions_all(AuctionsRequest {
            symbols: vec!["AAPL".to_owned()],
            start: Some("2026-07-01".to_owned()),
            end: Some("2026-07-10".to_owned()),
            limit: Some(2),
            asof: None,
            feed: Some(alpaca_data::stocks::AuctionFeed::Sip),
            currency: None,
            page_token: None,
            sort: Some(alpaca_data::stocks::Sort::Asc),
        })
        .await
        .expect("single-symbol auctions should paginate through the real Data API");

    let auctions = response
        .auctions
        .get("AAPL")
        .expect("canonical auctions response should contain AAPL");
    assert!(auctions.len() > 2);
    assert!(
        auctions.iter().all(|auction| auction.d.is_some()),
        "daily auction date should be present"
    );
    assert!(
        auctions.iter().any(|auction| !auction.o.is_empty()),
        "opening auctions should be present"
    );
    assert!(
        auctions.iter().any(|auction| !auction.c.is_empty()),
        "closing auctions should be present"
    );
    auctions
        .iter()
        .flat_map(|auction| auction.o.iter().chain(&auction.c))
        .for_each(assert_auction_shape);
    assert!(response.next_page_token.is_none());

    let attempts = observer.requests();
    let requests = unique_observed_requests(&attempts);
    let retries = observer.retries();
    let responses = observer.responses();
    assert!(
        responses.len() > 1,
        "limit=2 should exercise real auction pagination"
    );
    assert_eq!(attempts.len(), responses.len() + retries.len());
    assert_eq!(requests.len(), responses.len());
    assert!(requests.iter().all(|request| {
        request.operation.as_deref() == Some("stocks.auction_single")
            && request.url.contains("/v2/stocks/AAPL/auctions")
            && observed_query_value(request, "symbols").is_none()
            && observed_query_value(request, "feed").as_deref() == Some("sip")
    }));
    assert!(
        responses
            .iter()
            .all(|meta| { meta.status() == 200 && meta.request_id().is_some() })
    );
    eprintln!(
        "real_api operation=stocks.auction_single pages={} attempts={} retries={:?} requests={:?} statuses={:?} request_ids={:?} shape=symbol+auctions[](d,o[],c[])+next_page_token auctions={}",
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
        auctions.len()
    );
}

#[tokio::test]
async fn stocks_resource_reads_real_api_endpoints() {
    let (client, _) = real_data_client();
    let stocks = client.stocks();

    let latest_bars = stocks
        .latest_bars(LatestBarsRequest {
            symbols: vec!["AAPL".to_owned(), "MSFT".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("latest bars should read from real API");
    assert!(latest_bars.bars.contains_key("AAPL"));
    assert!(latest_bars.bars.contains_key("MSFT"));

    let latest_quotes = stocks
        .latest_quotes(LatestQuotesRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("latest quotes should read from real API");
    assert!(latest_quotes.quotes.contains_key("AAPL"));
    assert!(latest_quotes.quotes["AAPL"].t.is_some());

    let latest_trades = stocks
        .latest_trades(LatestTradesRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("latest trades should read from real API");
    assert!(latest_trades.trades.contains_key("AAPL"));
    assert!(latest_trades.trades["AAPL"].t.is_some());

    let snapshots = stocks
        .snapshots(SnapshotsRequest {
            symbols: vec!["AAPL".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("snapshots should read from real API");
    let snapshot = snapshots
        .get("AAPL")
        .expect("single-symbol snapshots should contain AAPL");
    assert!(snapshot.latest_trade.is_some() || snapshot.latest_quote.is_some());
    assert!(snapshot.timestamp().is_some());
    assert!(snapshot.price().is_some());
    assert!(snapshot.bid_price().is_some() || snapshot.ask_price().is_some());
    assert!(snapshot.session_close().is_some() || snapshot.previous_close().is_some());

    let batch_snapshots = stocks
        .snapshots(SnapshotsRequest {
            symbols: vec!["AAPL".to_owned(), "brk/b".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect("batch snapshots should absorb stock symbol normalization");
    assert!(batch_snapshots.contains_key("AAPL"));
    assert!(batch_snapshots.contains_key("BRK.B"));
    assert_eq!(display_stock_symbol("brk/b"), "BRK.B");
    let ordered = ordered_snapshots(&batch_snapshots);
    assert_eq!(ordered.len(), batch_snapshots.len());
    assert!(ordered.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    assert!(
        ordered.iter().all(|(_, snapshot)| {
            snapshot.timestamp().is_some()
                && snapshot.price().is_some()
                && (snapshot.bid_price().is_some() || snapshot.ask_price().is_some())
                && (snapshot.session_close().is_some() || snapshot.previous_close().is_some())
        }),
        "ordered stock snapshots should expose canonical quote/session readers"
    );

    let brk_snapshots = stocks
        .snapshots(SnapshotsRequest {
            symbols: vec!["brk/b".to_owned()],
            feed: Some(preferred_feed(false)),
            currency: None,
        })
        .await
        .expect(
            "BRK.B snapshots request should succeed through canonical stock symbol normalization",
        );
    let brk_snapshot = brk_snapshots
        .get("BRK.B")
        .expect("BRK.B snapshots should contain BRK.B");
    assert!(brk_snapshot.timestamp().is_some());
    assert!(brk_snapshot.price().is_some());
    assert!(brk_snapshot.bid_price().is_some() || brk_snapshot.ask_price().is_some());

    let bars = stocks
        .bars_all(BarsRequest {
            symbols: vec!["AAPL".to_owned()],
            timeframe: TimeFrame::day_1(),
            start: Some("2026-04-01T00:00:00Z".to_owned()),
            end: Some("2026-04-08T00:00:00Z".to_owned()),
            limit: Some(1),
            adjustment: None,
            feed: Some(preferred_feed(false)),
            sort: None,
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .expect("historical bars should paginate through real API");
    let aapl_bars = bars.bars.get("AAPL").expect("AAPL bars should exist");
    assert!(aapl_bars.len() > 1);
    assert!(aapl_bars.iter().all(|bar| bar.t.is_some()));

    let condition_codes = stocks
        .condition_codes(ConditionCodesRequest {
            ticktype: TickType::Trade,
            tape: Tape::C,
        })
        .await
        .expect("condition codes should read from real API");
    assert!(!condition_codes.is_empty());

    let exchange_codes = stocks
        .exchange_codes()
        .await
        .expect("exchange codes should read from real API");
    assert!(!exchange_codes.is_empty());
}
