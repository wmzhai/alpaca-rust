# Corporate Actions

`alpaca-data::Client::corporate_actions()` exposes the corporate-actions listing route.

## Implemented Methods

- `list`
- `list_all`

## Typical Request

```rust
use alpaca_data::{Client, corporate_actions};

let client = Client::from_env()?;
let actions = client
    .corporate_actions()
    .list(corporate_actions::ListRequest {
        symbols: Some(vec!["AAPL".into()]),
        start: Some("2026-01-01".into()),
        end: Some("2026-12-31".into()),
        ..corporate_actions::ListRequest::default()
    })
    .await?;
# let _ = actions;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- `ids` cannot be combined with the other official filter fields
- `symbols`, `cusips`, and `ids` must be non-empty when provided
- `symbols` are normalized through the stock display-symbol rules before the SDK sends the request
- `list_all` only expands pagination; it does not reinterpret the official payload

## Not Implemented Here

- corporate-action streaming or notification layers
- custom corporate-action normalization across providers
