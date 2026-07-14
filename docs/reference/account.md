# Account

`alpaca-trade::Client::account()` exposes account-level state.

## Implemented Methods

- `get`

Canonical operation: `getAccount` (`GET /v2/account`). Its Trading API `2.0.1`
contract is closed against both Paper and the standalone mock HTTP service.

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

## Contract Notes

- fields that are nullable in the canonical schema remain optional in the Rust model
- Paper fields observed outside the canonical schema are preserved as typed, optional extensions only when backed by real response evidence
- the mock response uses a stable UUID and does not derive an account identifier from the API key

## Not Implemented Here

- broker account APIs
- high-level portfolio analytics beyond the official response
