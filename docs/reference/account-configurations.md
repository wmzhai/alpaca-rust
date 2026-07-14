# Account Configurations

`alpaca-trade::Client::account_configurations()` exposes the configuration endpoints for the current trading account.

## Implemented Methods

- `get`
- `update`

Canonical operations `getAccountConfig` and `patchAccountConfig` are closed
against both Paper and the standalone mock HTTP service.

## Typical Request

```rust
use alpaca_trade::{Client, account_configurations};

let client = Client::from_env()?;
let updated = client
    .account_configurations()
    .update(account_configurations::UpdateRequest {
        no_shorting: Some(false),
        ..account_configurations::UpdateRequest::default()
    })
    .await?;
# let _ = updated;
# Ok::<(), alpaca_trade::Error>(())
```

## Not Implemented Here

- policy abstractions beyond the official mutable fields
- staged configuration orchestration

## Contract Notes

- updates are partial; only fields set to `Some(...)` are sent
- the Paper validation scenario reads the current `trade_confirm_email`, writes that same value, and reads it again, so validation does not change account policy
- `closing_transactions_only` is retained as an optional Paper-observed response extension even though it is absent from the canonical response schema
