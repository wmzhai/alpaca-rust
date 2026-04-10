# Watchlists

`alpaca-trade::Client::watchlists()` exposes the full adopted watchlist resource family.

## Implemented Methods

- `list`
- `create`
- `get_by_id`
- `update_by_id`
- `delete_by_id`
- `add_asset_by_id`
- `delete_symbol_by_id`
- `get_by_name`
- `update_by_name`
- `add_asset_by_name`
- `delete_by_name`

## Typical Request

```rust
use alpaca_trade::{Client, watchlists};

let client = Client::from_env()?;
let watchlist = client
    .watchlists()
    .create(watchlists::CreateRequest {
        name: "core-tech".into(),
        symbols: Some(vec!["AAPL".into(), "MSFT".into()]),
    })
    .await?;
# let _ = watchlist;
# Ok::<(), alpaca_trade::Error>(())
```

## Notes

- both id-based and name-based official routes are supported
- this crate keeps the official route split instead of collapsing it into a custom abstraction
