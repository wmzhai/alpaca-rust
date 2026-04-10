# Portfolio History

`alpaca-trade::Client::portfolio_history()` exposes the account portfolio-history route.

## Implemented Methods

- `get`

## Typical Request

```rust
use alpaca_trade::{Client, portfolio_history};

let client = Client::from_env()?;
let history = client
    .portfolio_history()
    .get(portfolio_history::GetRequest {
        period: Some("1M".into()),
        timeframe: Some("1D".into()),
        ..portfolio_history::GetRequest::default()
    })
    .await?;
# let _ = history;
# Ok::<(), alpaca_trade::Error>(())
```

## Notes

- timestamp arrays follow the official API contract
- this crate does not convert the public timestamp fields into richer time types automatically
