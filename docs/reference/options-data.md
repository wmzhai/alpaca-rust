# Options Market Data

`alpaca-data::Client::options()` exposes Alpaca option market-data HTTP routes.

## Implemented Mirror Methods

- `bars`
- `trades`
- `latest_quotes`
- `latest_trades`
- `snapshots`
- `chain`
- `condition_codes`
- `exchange_codes`

## Convenience Methods

- `bars_all`
- `trades_all`
- `snapshots_all`
- `chain_all`

## Typical Requests

Historical bars for concrete option symbols:

```rust
use alpaca_data::{Client, options};

let client = Client::from_env()?;
let bars = client
    .options()
    .bars(options::BarsRequest {
        symbols: vec!["AAPL260619C00200000".into()],
        timeframe: options::TimeFrame::OneDay,
        start: Some("2026-04-01T00:00:00Z".into()),
        end: Some("2026-04-08T00:00:00Z".into()),
        ..options::BarsRequest::default()
    })
    .await?;
# let _ = bars;
# Ok::<(), alpaca_data::Error>(())
```

Option chain snapshots by underlying:

```rust
use alpaca_data::{Client, options};

let client = Client::from_env()?;
let chain = client
    .options()
    .chain(options::ChainRequest {
        underlying_symbol: "AAPL".into(),
        expiration_date_gte: Some("2026-06-01".into()),
        expiration_date_lte: Some("2026-06-30".into()),
        ..options::ChainRequest::default()
    })
    .await?;
# let _ = chain;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- historical and latest symbol-list routes require non-empty option symbol lists
- chain queries require a non-empty `underlying_symbol`
- chain and snapshot endpoints honor official pagination and limit bounds
- numeric strike filters stay exact through `rust_decimal::Decimal`

## Not Implemented Here

- option websocket or streaming APIs
- option pricing, Greeks, or implied-volatility helpers
- synthetic contract discovery beyond the official HTTP routes
