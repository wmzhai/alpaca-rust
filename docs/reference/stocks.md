# Stocks

`alpaca-data::Client::stocks()` exposes the stock market-data resource family.

## Implemented Mirror Methods

- `bars`
- `quotes`
- `trades`
- `latest_bars`
- `latest_quotes`
- `latest_trades`
- `snapshots`
- `auctions`
- `condition_codes`
- `exchange_codes`

## Convenience Methods

- `bars_all`
- `quotes_all`
- `trades_all`
- `auctions_all`

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

Use the canonical latest methods for either one or multiple symbols:

```rust
use alpaca_data::{Client, stocks};

let client = Client::from_env()?;
let latest = client
    .stocks()
    .latest_quotes(stocks::LatestQuotesRequest {
        symbols: vec!["AAPL".into()],
        ..stocks::LatestQuotesRequest::default()
    })
    .await?;
# let _ = latest;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- historical multi-symbol endpoints require a non-empty `symbols` list
- latest and snapshot reads also use the canonical batch request types, including single-symbol calls
- `latest_bars` dispatches one-symbol requests to the official `/v2/stocks/{symbol}/bars/latest` route and normalizes the wire response back into `LatestBarsResponse`
- `bars` / `bars_all` dispatch one-symbol requests and pagination to `/v2/stocks/{symbol}/bars`, while multi-symbol requests continue to use `/v2/stocks/bars`
- `latest_quotes` dispatches one-symbol requests to `/v2/stocks/{symbol}/quotes/latest` and preserves the canonical map response
- `quotes` / `quotes_all` dispatch one-symbol requests and pagination to `/v2/stocks/{symbol}/quotes`, while multi-symbol requests continue to use `/v2/stocks/quotes`
- `latest_trades` dispatches one-symbol requests to `/v2/stocks/{symbol}/trades/latest` and preserves the canonical map response
- `trades` / `trades_all` dispatch one-symbol requests and pagination to `/v2/stocks/{symbol}/trades`, while multi-symbol requests continue to use `/v2/stocks/trades`
- `snapshots` dispatches one-symbol requests to `/v2/stocks/{symbol}/snapshot` and normalizes the wire response into the existing symbol-keyed map
- `auctions` / `auctions_all` dispatch one-symbol requests and pagination to `/v2/stocks/{symbol}/auctions`; the official route only supports the SIP auction feed
- historical endpoints support `feed`, `sort`, `asof`, `currency`, and pagination where the official route supports them
- `limit` validation follows the official endpoint contract instead of silently auto-chunking requests

## Not Implemented Here

- stock websocket or streaming APIs
- any cross-provider normalization layer
- caching, subscription, or application-side state management
