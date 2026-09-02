# alpaca-data

`alpaca-data` is an async Rust client for the Alpaca Market Data HTTP API.

## Current Coverage

- stocks
- options
- crypto
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
- `client.crypto()`
- `client.news()`
- `client.corporate_actions()`

## Raw Cache Convenience

```rust
use alpaca_data::cache::CachedClient;
use alpaca_data::Client;

let raw = Client::builder()
    .credentials_from_env()?
    .build()?;
let cache = CachedClient::new(raw);
# let _ = cache;
# Ok::<(), alpaca_data::Error>(())
```

`alpaca_data::cache::CachedClient` is an opt-in convenience facade for:

- cache-first stock snapshots
- cache-first raw option snapshots
- explicit stock bar subscriptions
- explicit refresh / clear operations

It does not own scheduling, business clocks, option-chain enrichment, or IV / Greeks calculations.

Stock and raw option snapshot caches share one requested-set reconciliation contract:

- provider errors are returned without changing cached values, unavailable keys, or the last successful refresh time
- successful full, partial, and empty responses reconcile only the keys captured for that request
- returned requested keys replace cached values and leave the unavailable set, while requested-but-omitted keys lose any old value and enter the unavailable set
- unexpected response keys are ignored, so an in-flight request cannot overwrite unrelated subscriptions
- a non-empty successful provider request advances the receive timestamp even when every requested key is omitted; an empty subscription set does not call the provider or advance time

`CacheStats` exposes subscribed, cached, unavailable, and last-success counts/timestamps so callers can distinguish complete, partial, empty, and failed refreshes.

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

### Crypto

- `bars` / `bars_all`
- `quotes` / `quotes_all`
- `trades` / `trades_all`
- `latest_bars`
- `latest_quotes`
- `latest_trades`
- `latest_orderbooks`
- `snapshots`

### News

- `list`
- `list_all`

### Corporate Actions

- `list`
- `list_all`

## Built-in Convenience Helpers

- `stocks::ordered_snapshots(...)`
- `options::ordered_snapshots(...)`
- `crypto::ordered_snapshots(...)`
- `crypto::preferred_location()`
- `crypto::Snapshot::{timestamp, price, last_price, bid_price, ask_price, mark_price}`
- `stocks::Snapshot::{timestamp, price, bid_price, ask_price, session_open, session_high, session_low, session_close, previous_close, session_volume}`
- `options::Snapshot::{timestamp, bid_price, ask_price, last_price, mark_price}`
- `options::underlying_symbol(...)`
- `symbols::display_stock_symbol(...)`

## Not Implemented

- crypto perpetual futures
- forex
- fixed income
- index
- logos
- screener
- websocket / stream APIs

## Environment Variables

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

See `docs/reference/alpaca-data.md` and <https://docs.rs/alpaca-data> for the full reference.
