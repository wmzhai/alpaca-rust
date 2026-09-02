# Crypto

`alpaca-data::Client::crypto()` exposes the crypto spot market-data resource family.

## Implemented Mirror Methods

- `bars`
- `quotes`
- `trades`
- `latest_bars`
- `latest_quotes`
- `latest_trades`
- `latest_orderbooks`
- `snapshots`

## Convenience Methods

- `bars_all`
- `quotes_all`
- `trades_all`
- `preferred_location()`
- `ordered_snapshots(...)`
- `Snapshot::{timestamp, price, last_price, bid_price, ask_price, mark_price}`

## Typical Requests

Use `BarsRequest` for historical multi-symbol bars. The default location is Alpaca US:

```rust
use alpaca_data::{Client, crypto};

let client = Client::from_env()?;
let response = client
    .crypto()
    .bars(crypto::BarsRequest {
        location: crypto::preferred_location(),
        symbols: vec!["BTC/USD".into(), "ETH/USD".into()],
        timeframe: crypto::TimeFrame::day_1(),
        start: Some("2026-08-01T00:00:00Z".into()),
        end: Some("2026-08-08T00:00:00Z".into()),
        ..crypto::BarsRequest::default()
    })
    .await?;
# let _ = response;
# Ok::<(), alpaca_data::Error>(())
```

Latest quotes, trades, bars, orderbooks, and snapshots all use the same batch request shape:

```rust
use alpaca_data::{Client, crypto};

let client = Client::from_env()?;
let latest = client
    .crypto()
    .latest_quotes(crypto::LatestQuotesRequest {
        location: crypto::preferred_location(),
        symbols: vec!["BTC/USD".into()],
    })
    .await?;
# let _ = latest;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- all routes are `/v1beta3/crypto/{loc}/...`; there are no single-symbol crypto paths
- `preferred_location()` is `us` (Alpaca US). Official locations also include `us-1`, `us-2`, `eu-1`, and `bs-1`
- symbols use slash pairs such as `BTC/USD`
- crypto volume and quote/trade sizes are fractional and map to `Decimal`
- historical endpoints support `start`, `end`, `limit`, `sort`, and pagination
- `limit` validation follows the official endpoint contract
- crypto bars may have quote-mid prices with `v = 0` when no trade occurs in the interval

## Not Implemented Here

- crypto perpetual futures market data
- websocket / stream APIs
- cache, facade, or mock coverage
