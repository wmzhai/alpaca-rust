# Assets

`alpaca-trade::Client::assets()` exposes tradable asset metadata.

## Implemented Methods

- `list`
- `get`

Canonical operations `get-v2-assets` and
`get-v2-assets-symbol_or_asset_id` are implemented by `alpaca-trade`.
The standalone mock forwards both requests to the official Trading API and
returns the upstream status and body unchanged. Keys prefixed with `PK` use
Paper; every other key uses the live Trading host.

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
- omitted optional fields, including `cusip`, `borrow_status`, and `attributes`, deserialize as absent
- get accepts either a symbol or asset ID
