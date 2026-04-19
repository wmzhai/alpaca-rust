# alpaca-data

`alpaca-data` is the async Rust Market Data HTTP client in the `alpaca-rust` workspace.

## Current Coverage

### Implemented Resource Families

- stocks
- options
- news
- corporate actions

### Not Implemented

- crypto
- forex
- fixed income
- logos
- screener
- websocket / stream APIs

## Client Entry

### Construction

- `Client::builder()`
- `Client::new(credentials)`
- `Client::from_env()`

### Common Builder Methods

- `credentials(...)`
- `api_key(...)`
- `secret_key(...)`
- `base_url(...)`
- `base_url_str(...)`
- `credentials_from_env(...)`
- `credentials_from_env_names(...)`
- `base_url_from_env(...)`
- `timeout(...)`
- `observer(...)`
- `retry_config(...)`
- `max_in_flight(...)`
- `build()`

### Resource Accessors

- `client.stocks()`
- `client.options()`
- `client.news()`
- `client.corporate_actions()`

## Stocks API

### Resource Client

- `stocks().bars(...)`
- `stocks().bars_all(...)`
- `stocks().auctions(...)`
- `stocks().auctions_all(...)`
- `stocks().quotes(...)`
- `stocks().quotes_all(...)`
- `stocks().trades(...)`
- `stocks().trades_all(...)`
- `stocks().latest_bars(...)`
- `stocks().latest_quotes(...)`
- `stocks().latest_trades(...)`
- `stocks().snapshots(...)`
- `stocks().condition_codes(...)`
- `stocks().exchange_codes(...)`

### Convenience Helpers

- `stocks::ordered_snapshots(...)`
- `stocks::Snapshot::{timestamp, price, bid_price, ask_price, session_open, session_high, session_low, session_close, previous_close, session_volume}`
- provider-safe stock symbol normalization is absorbed directly by the canonical batch request shapes

## Options API

### Resource Client

- `options().bars(...)`
- `options().bars_all(...)`
- `options().trades(...)`
- `options().trades_all(...)`
- `options().latest_quotes(...)`
- `options().latest_trades(...)`
- `options().snapshots(...)`
- `options().snapshots_all(...)`
- `options().chain(...)`
- `options().chain_all(...)`
- `options().condition_codes(...)`
- `options().exchange_codes(...)`

### Convenience Helpers

- `options::ordered_snapshots(...)`
- `options::Snapshot::{timestamp, bid_price, ask_price, last_price, mark_price}`
- `options::underlying_symbol(...)`

Notes:

- `snapshots_all(...)` absorbs Alpaca's current single-request limit of `100` contracts
- `underlying_symbol(...)` normalizes inputs such as `BRK.B -> BRKB` into provider form

## News API

### Resource Client

- `news().list(...)`
- `news().list_all(...)`

## Corporate Actions API

### Resource Client

- `corporate_actions().list(...)`
- `corporate_actions().list_all(...)`

### Convenience Helpers

- `corporate_actions::ListRequest` absorbs provider-safe stock symbol normalization directly

## Shared Helpers

- `symbols::options_underlying_symbol(...)`
- `symbols::display_stock_symbol(...)`
- `pagination::collect_all(...)`

## Environment Variables

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- `ALPACA_DATA_BASE_URL`

## Related Documents

- [Stocks](./stocks.md)
- [Options Market Data](./options-data.md)
- [News](./news.md)
- [Corporate Actions](./corporate-actions.md)
- [Market Data API Coverage](../api-coverage/market-data.md)
- [docs.rs/alpaca-data](https://docs.rs/alpaca-data)
