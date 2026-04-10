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
