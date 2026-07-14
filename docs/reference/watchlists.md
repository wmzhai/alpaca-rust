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

All 11 canonical Watchlist operations are closed against both Paper and the
standalone mock HTTP service.

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
- create accepts an omitted or empty symbol list
- update is partial: it accepts name-only or symbols-only changes, and an empty symbol list clears the watchlist
- Paper rejects a `null` symbol item even though the canonical array item is nullable, so the Rust request uses `Vec<String>` rather than `Vec<Option<String>>`
- returned `assets` remains optional because it is not required by the canonical schema; successful Paper/mock lifecycle responses include an array
- the mock keeps per-account ordered state, unique names, ID/name lookup, and deletion behavior over the same HTTP routes
