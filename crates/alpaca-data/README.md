# alpaca-data

`alpaca-data` is an async Rust client for the Alpaca Market Data HTTP API.

Covered resource families in the current release line:

- stocks
- options
- news
- corporate actions

Implemented mirror operations:

- stocks: `bars`, `quotes`, `trades`, `latest_bars`, `latest_quotes`, `latest_trades`, `snapshots`, `auctions`, `condition_codes`, `exchange_codes`
- options: `bars`, `trades`, `latest_quotes`, `latest_trades`, `snapshots`, `chain`, `condition_codes`, `exchange_codes`
- news: `list`
- corporate actions: `list`

Convenience helpers:

- stocks: `bars_all`, `quotes_all`, `trades_all`, `auctions_all`
- stocks snapshots: canonical `timestamp()` / `price()` readers plus `ordered_snapshots(...)`
- stocks requests: provider-safe stock symbol normalization is absorbed by the canonical batch request types
- options: `bars_all`, `trades_all`, `snapshots_all`, `chain_all`
- options snapshots: canonical `timestamp()` / `bid_price()` / `ask_price()` / `last_price()` / `mark_price()` readers plus `ordered_snapshots(...)`
- news: `list_all`
- corporate actions: `list_all`
- corporate actions requests: stock symbol normalization is absorbed by `ListRequest`

Not implemented in the current release line:

- crypto
- forex
- fixed income
- logos
- screener
- websocket
- stream APIs

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
