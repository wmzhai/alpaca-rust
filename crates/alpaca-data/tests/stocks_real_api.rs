#[path = "../../../tests/support/live/mod.rs"]
mod live_support;

use alpaca_data::{
    Client,
    stocks::{
        BarsRequest, ConditionCodesRequest, DataFeed, LatestBarsRequest, LatestQuoteRequest,
        LatestTradeRequest, SnapshotRequest, SnapshotsRequest, Tape, TickType, TimeFrame,
        display_symbol, ordered_snapshots,
    },
};
use live_support::{AlpacaService, LiveTestEnv, SampleRecorder};

#[tokio::test]
async fn stocks_resource_reads_real_api_endpoints() {
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
    let stocks = client.stocks();
    let recorder = SampleRecorder::from_live_env(&env);

    let latest_bars = stocks
        .latest_bars(LatestBarsRequest {
            symbols: vec!["AAPL".to_owned(), "MSFT".to_owned()],
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("latest bars should read from real API");
    recorder
        .record_json("alpaca-data-stocks", "latest-bars", &latest_bars)
        .expect("latest bars sample should record");
    assert!(latest_bars.bars.contains_key("AAPL"));
    assert!(latest_bars.bars.contains_key("MSFT"));

    let latest_quote = stocks
        .latest_quote(LatestQuoteRequest {
            symbol: "AAPL".to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("latest quote should read from real API");
    recorder
        .record_json("alpaca-data-stocks", "latest-quote", &latest_quote)
        .expect("latest quote sample should record");
    assert_eq!(latest_quote.symbol, "AAPL");
    assert!(latest_quote.quote.t.is_some());

    let latest_trade = stocks
        .latest_trade(LatestTradeRequest {
            symbol: "AAPL".to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("latest trade should read from real API");
    recorder
        .record_json("alpaca-data-stocks", "latest-trade", &latest_trade)
        .expect("latest trade sample should record");
    assert_eq!(latest_trade.symbol, "AAPL");
    assert!(latest_trade.trade.t.is_some());

    let snapshot = stocks
        .snapshot(SnapshotRequest {
            symbol: "AAPL".to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("snapshot should read from real API");
    recorder
        .record_json("alpaca-data-stocks", "snapshot", &snapshot)
        .expect("snapshot sample should record");
    assert_eq!(snapshot.symbol, "AAPL");
    assert!(snapshot.latest_trade.is_some() || snapshot.latest_quote.is_some());
    assert!(snapshot.timestamp().is_some());
    assert!(snapshot.price().is_some());

    let batch_snapshots = stocks
        .snapshots(SnapshotsRequest {
            symbols: vec!["AAPL".to_owned(), "brk/b".to_owned()],
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("batch snapshots should absorb stock symbol normalization");
    recorder
        .record_json("alpaca-data-stocks", "snapshots", &batch_snapshots)
        .expect("snapshots sample should record");
    assert!(batch_snapshots.contains_key("AAPL"));
    assert!(batch_snapshots.contains_key("BRK.B"));
    assert_eq!(display_symbol("brk/b"), "BRK.B");
    let ordered = ordered_snapshots(&batch_snapshots);
    assert_eq!(ordered.len(), batch_snapshots.len());
    assert!(ordered.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    assert!(
        ordered
            .iter()
            .all(|(_, snapshot)| snapshot.timestamp().is_some() && snapshot.price().is_some()),
        "ordered stock snapshots should expose canonical timestamp and price helpers"
    );

    let brk_snapshot = stocks
        .snapshot(SnapshotRequest {
            symbol: "brk/b".to_owned(),
            feed: Some(DataFeed::Iex),
            currency: None,
        })
        .await
        .expect("BRK.B snapshot request should succeed through canonical stock symbol normalization");
    assert_eq!(brk_snapshot.symbol, "BRK.B");
    assert!(brk_snapshot.timestamp().is_some());
    assert!(brk_snapshot.price().is_some());

    let bars = stocks
        .bars_all(BarsRequest {
            symbols: vec!["AAPL".to_owned()],
            timeframe: TimeFrame::day_1(),
            start: Some("2026-04-01T00:00:00Z".to_owned()),
            end: Some("2026-04-08T00:00:00Z".to_owned()),
            limit: Some(1),
            adjustment: None,
            feed: Some(DataFeed::Iex),
            sort: None,
            asof: None,
            currency: None,
            page_token: None,
        })
        .await
        .expect("historical bars should paginate through real API");
    recorder
        .record_json("alpaca-data-stocks", "bars-all", &bars)
        .expect("bars sample should record");
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
