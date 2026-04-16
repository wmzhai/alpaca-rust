# alpaca-trade

`alpaca-trade` is an async Rust client for the Alpaca Trading HTTP API.

## Current Coverage

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

## Client Entry

```rust
use alpaca_trade::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_trade::Error>(())
```

The default builder targets Alpaca paper trading. Use `Client::builder().live()` for the live environment.

### Resource Accessors

- `client.account()`
- `client.account_configurations()`
- `client.activities()`
- `client.assets()`
- `client.calendar()`
- `client.clock()`
- `client.options_contracts()`
- `client.orders()`
- `client.portfolio_history()`
- `client.positions()`
- `client.watchlists()`

## Main API Surface

### Account / Configurations

- `account().get()`
- `account_configurations().get()`
- `account_configurations().update(...)`

### Activities / Assets

- `activities().list(...)`
- `activities().list_all(...)`
- `assets().list(...)`
- `assets().get(...)`

### Calendar / Clock

- `calendar().list(...)`
- `calendar().list_v3(...)`
- `clock().get()`
- `clock().get_v3(...)`

### Options Contracts

- `options_contracts().list(...)`
- `options_contracts().list_all(...)`
- `options_contracts().get(...)`

### Orders

- `orders().list(...)`
- `orders().create(...)`
- `orders().cancel_all()`
- `orders().get(...)`
- `orders().replace(...)`
- `orders().cancel(...)`
- `orders().get_by_client_order_id(...)`

### Portfolio / Positions / Watchlists

- `portfolio_history().get(...)`
- `positions().list()`
- `positions().get(...)`
- `positions().close_all(...)`
- `positions().close(...)`
- `positions().exercise(...)`
- `positions().do_not_exercise(...)`
- `watchlists().list()`
- `watchlists().create(...)`
- `watchlists().get_by_id(...)`
- `watchlists().update_by_id(...)`
- `watchlists().delete_by_id(...)`
- `watchlists().add_asset_by_id(...)`
- `watchlists().delete_symbol_by_id(...)`
- `watchlists().get_by_name(...)`
- `watchlists().update_by_name(...)`
- `watchlists().add_asset_by_name(...)`
- `watchlists().delete_by_name(...)`

## Not Implemented

- websocket / stream APIs
- broker APIs
- FIX
- crypto / fixed-income trading surfaces
- high-level order orchestration

## Environment Variables

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- `ALPACA_TRADE_BASE_URL`

See `docs/reference/alpaca-trade.md` and <https://docs.rs/alpaca-trade> for the full reference.
