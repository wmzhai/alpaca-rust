# Options Contracts

`alpaca-trade::Client::options_contracts()` exposes the trading-side options contract catalog.

## Implemented Mirror Methods

- `list`
- `get`

## Convenience Methods

- `list_all`

## Typical Request

```rust
use alpaca_trade::{Client, options_contracts};

let client = Client::from_env()?;
let contracts = client
    .options_contracts()
    .list(options_contracts::ListRequest {
        underlying_symbols: Some(vec!["AAPL".into()]),
        limit: Some(100),
        ..options_contracts::ListRequest::default()
    })
    .await?;
# let _ = contracts;
# Ok::<(), alpaca_trade::Error>(())
```

## Request Notes

- `underlying_symbols` must be non-empty when provided
- `limit` follows the official route bounds
- strike filters use `rust_decimal::Decimal`

## Not Implemented Here

- option pricing, Greeks, or analytics helpers
- multi-provider contract normalization
