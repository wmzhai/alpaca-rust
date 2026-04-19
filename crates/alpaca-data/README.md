# alpaca-data

`alpaca-data` is an async Rust client for the Alpaca Market Data HTTP API.

## Current Coverage

- stocks
- options
- news
- corporate actions

## Client Entry

```rust
use alpaca_data::Client;

let client = Client::builder()
    .credentials_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_data::Error>(())
```

### Resource Accessors

- `client.stocks()`
- `client.options()`
- `client.news()`
- `client.corporate_actions()`

## Main API Surface

### Stocks

- `bars` / `bars_all`
- `auctions` / `auctions_all`
- `quotes` / `quotes_all`
- `trades` / `trades_all`
- `latest_bars`
- `latest_quotes`
- `latest_trades`
- `snapshots`
- `condition_codes`
- `exchange_codes`

### Options

- `bars` / `bars_all`
- `trades` / `trades_all`
- `latest_quotes`
- `latest_trades`
- `snapshots` / `snapshots_all`
- `chain` / `chain_all`
- `condition_codes`
- `exchange_codes`

### News

- `list`
- `list_all`

### Corporate Actions

- `list`
- `list_all`

## Built-in Convenience Helpers

- `stocks::ordered_snapshots(...)`
- `options::ordered_snapshots(...)`
- `stocks::Snapshot::{timestamp, price, bid_price, ask_price, session_open, session_high, session_low, session_close, previous_close, session_volume}`
- `options::Snapshot::{timestamp, bid_price, ask_price, last_price, mark_price}`
- `options::underlying_symbol(...)`
- `symbols::display_stock_symbol(...)`

## Not Implemented

- crypto
- forex
- fixed income
- logos
- screener
- websocket / stream APIs

## Environment Variables

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

See `docs/reference/alpaca-data.md` and <https://docs.rs/alpaca-data> for the full reference.
