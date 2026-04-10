# Account Configurations

`alpaca-trade::Client::account_configurations()` exposes the configuration endpoints for the current trading account.

## Implemented Methods

- `get`
- `update`

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
