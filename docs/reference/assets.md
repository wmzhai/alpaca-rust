# Assets

`alpaca-trade::Client::assets()` exposes tradable asset metadata.

## Implemented Methods

- `list`
- `get`

## Typical Request

```rust
use alpaca_trade::{Client, assets};

let client = Client::from_env()?;
let active = client
    .assets()
    .list(assets::ListRequest {
        status: Some("active".into()),
        ..assets::ListRequest::default()
    })
    .await?;
# let _ = active;
# Ok::<(), alpaca_trade::Error>(())
```

## Not Implemented Here

- broker catalog APIs
- cross-provider asset taxonomy normalization
