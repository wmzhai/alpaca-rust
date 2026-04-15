# alpaca-data

`alpaca-data` is an async Rust client for the Alpaca Market Data HTTP API.

Covered resource families in the current release line:

- stocks
- options
- news
- corporate actions

Implemented mirror operations:

- stocks: `bars`, `bars_single`, `quotes`, `quotes_single`, `trades`, `trades_single`, `latest_bars`, `latest_bar`, `latest_quotes`, `latest_quote`, `latest_trades`, `latest_trade`, `snapshots`, `snapshot`, `auctions`, `auctions_single`, `condition_codes`, `exchange_codes`
- options: `bars`, `trades`, `latest_quotes`, `latest_trades`, `snapshots`, `chain`, `condition_codes`, `exchange_codes`
- news: `list`
- corporate actions: `list`

Convenience helpers:

- stocks: `bars_all`, `bars_single_all`, `quotes_all`, `quotes_single_all`, `trades_all`, `trades_single_all`, `auctions_all`, `auctions_single_all`
- stocks snapshots: canonical `timestamp()` / `price()` readers plus `ordered_snapshots(...)`
- stocks requests: provider-safe stock symbol normalization for batch and single-symbol reads
- options: `bars_all`, `trades_all`, `snapshots_all`, `chain_all`
- news: `list_all`
- corporate actions: `list_all`

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
