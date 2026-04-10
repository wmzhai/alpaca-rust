# alpaca-data

`alpaca-data` is an async Rust client for the Alpaca Market Data HTTP API.

Covered resource families in the current release line:

- stocks
- options
- news
- corporate actions

Environment variables:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- `ALPACA_DATA_BASE_URL`

Quick start:

```rust
use alpaca_data::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_data::Error>(())
```

The public docs site for the workspace lives at <https://wmzhai.github.io/alpaca-rust/>.
