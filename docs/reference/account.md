# Account

`alpaca-trade::Client::account()` exposes account-level state.

## Implemented Methods

- `get`

## Typical Request

```rust
use alpaca_trade::Client;

let client = Client::from_env()?;
let account = client.account().get().await?;
# let _ = account;
# Ok::<(), alpaca_trade::Error>(())
```

## Related Resources

- `account_configurations()` for mutable configuration fields
- `portfolio_history()` for time-series portfolio data
- `activities()` for account activity history

## Not Implemented Here

- broker account APIs
- high-level portfolio analytics beyond the official response
