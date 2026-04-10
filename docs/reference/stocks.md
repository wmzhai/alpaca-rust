# Stocks

`alpaca-data::Client::stocks()` exposes the stock market-data resource family.

## Implemented Mirror Methods

- `bars`
- `bars_single`
- `quotes`
- `quotes_single`
- `trades`
- `trades_single`
- `latest_bars`
- `latest_bar`
- `latest_quotes`
- `latest_quote`
- `latest_trades`
- `latest_trade`
- `snapshots`
- `snapshot`
- `auctions`
- `auctions_single`
- `condition_codes`
- `exchange_codes`

## Convenience Methods

- `bars_all`
- `bars_single_all`
- `quotes_all`
- `quotes_single_all`
- `trades_all`
- `trades_single_all`
- `auctions_all`
- `auctions_single_all`

## Typical Requests

Use `BarsRequest` when you need historical multi-symbol bars:

```rust
use alpaca_data::{Client, stocks};

let client = Client::from_env()?;
let response = client
    .stocks()
    .bars(stocks::BarsRequest {
        symbols: vec!["AAPL".into(), "MSFT".into()],
        timeframe: stocks::TimeFrame::OneDay,
        start: Some("2026-04-01T00:00:00Z".into()),
        end: Some("2026-04-08T00:00:00Z".into()),
        ..stocks::BarsRequest::default()
    })
    .await?;
# let _ = response;
# Ok::<(), alpaca_data::Error>(())
```

Use `LatestQuoteRequest` or `LatestTradeRequest` for single-symbol latest endpoints:

```rust
use alpaca_data::{Client, stocks};

let client = Client::from_env()?;
let latest = client
    .stocks()
    .latest_quote(stocks::LatestQuoteRequest {
        symbol: "AAPL".into(),
        ..stocks::LatestQuoteRequest::default()
    })
    .await?;
# let _ = latest;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- historical multi-symbol endpoints require a non-empty `symbols` list
- single-symbol endpoints require a non-empty `symbol`
- historical endpoints support `feed`, `sort`, `asof`, `currency`, and pagination where the official route supports them
- `limit` validation follows the official endpoint contract instead of silently auto-chunking requests

## Not Implemented Here

- stock websocket or streaming APIs
- any cross-provider normalization layer
- caching, subscription, or application-side state management
