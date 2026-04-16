# alpaca-trade

`alpaca-trade` is an async Rust client for the Alpaca Trading HTTP API.

Covered resource families in the current release line:

- account
- account configurations
- activities
- assets
- calendar
- clock
- options contracts
- orders
- portfolio history
- positions
- watchlists

Implemented mirror operations:

- account: `get`
- account configurations: `get`, `update`
- activities: `list`
- assets: `list`, `get`
- calendar and clock: `list`, `list_v3`, `get`, `get_v3`
- options contracts: `list`, `get`
- orders: `list`, `create`, `cancel_all`, `get`, `replace`, `cancel`, `get_by_client_order_id`
- portfolio history: `get`
- positions: `list`, `get`, `close_all`, `close`, `exercise`, `do_not_exercise`
- watchlists: `list`, `create`, `get_by_id`, `update_by_id`, `delete_by_id`, `add_asset_by_id`, `delete_symbol_by_id`, `get_by_name`, `update_by_name`, `add_asset_by_name`, `delete_by_name`

Convenience helpers:

- activities: `list_all`
- options contracts: `list_all`

Not implemented in the current release line:

- websocket or stream APIs
- broker APIs
- FIX
- crypto or fixed-income trading surfaces
- high-level order workflows, strategy logic, or portfolio orchestration

Environment variables:

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- `ALPACA_TRADE_BASE_URL`

By default the client targets Alpaca paper trading. Use `Client::builder().live()` for the live base URL or `base_url_str(...)` for a custom endpoint.

```rust
use alpaca_trade::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_trade::Error>(())
```

For mock-backed end-to-end flows, see `alpaca-mock` and `docs/trade-mainline.md` in the repository.
