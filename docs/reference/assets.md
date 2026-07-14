# Assets

`alpaca-trade::Client::assets()` exposes tradable asset metadata.

## Implemented Methods

- `list`
- `get`

Canonical operations `get-v2-assets` and
`get-v2-assets-symbol_or_asset_id` are closed against both Paper and the
standalone mock HTTP service.

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

## Contract Notes

- status, asset class, exchange, attributes, and borrow status use typed values
- list filters include status, class, exchange, and attributes
- the response model includes the canonical order-size, trade-increment, and price-increment fields
- get accepts either a symbol or asset ID; the mock lookup treats symbols case-insensitively
