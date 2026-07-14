# Portfolio History

`alpaca-trade::Client::portfolio_history()` exposes the account portfolio-history route.

## Implemented Methods

- `get`

Canonical operation `getAccountPortfolioHistory` is closed against both Paper
and the standalone mock HTTP service.

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
- `timeframe`, `intraday_reporting`, and `pnl_reset` use typed request enums
- at most two of `period`, `start`, and `end` may be provided
- timestamp, equity, profit/loss, and profit/loss-percent arrays are expected to stay aligned
- this crate does not convert the public timestamp fields into richer time types automatically
