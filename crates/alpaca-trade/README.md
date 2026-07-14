# alpaca-trade

`alpaca-trade` is an async Rust client for the Alpaca Trading HTTP API.

## Current Coverage

The adopted Alpaca Trading API `2.0.1` surface contains 38 operations from the
68-operation canonical specification. All 38 have public client methods. The
current contract-validation checkpoint is 37 closed operations and 1 pending
operation:

- `optionDoNotExercise`

"Closed" means that the same public client scenario passed against both the
canonical Paper endpoint and an independently running `alpaca-mock` HTTP
server. A method being present does not by itself mean that its current
`2.0.1` contract has reached that checkpoint.

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
- `activities().list_by_type(...)`
- `activities().list_all(...)`
- `activities().list_option_activity_records(...)`
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
- `orders().create_resolved(...)`
- `orders().get_effective(...)`
- `orders().wait_for(...)`
- `orders().cancel_resolved(...)`
- `orders().replace_resolved(...)`
- `orders().submit_with_policy(...)`
- `orders().submit_resolved(...)`
- `orders().close_option_legs(...)`
- `orders().recover_market_close(...)`
- `orders().transition_resolved(...)`

All adopted order operations, including cancel-all and cancel by order ID, are
closed at the current checkpoint.

### Portfolio / Positions / Watchlists

- `portfolio_history().get(...)`
- `positions().list()`
- `positions().option_qty_map()`
- `positions().structure_quantity(...)`
- `positions().reconcile_signed_positions(...)`
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

Position list/get/close and exercise operations, plus all watchlist operations,
are closed at the current checkpoint. Exercise requires status `200` and
returns `ExerciseAccepted`: `details` is `None` for the canonical empty body or
contains typed `qty_exercised` and `qty_remaining` values for the JSON body
observed from Paper.

Do-not-exercise strictly requires an empty `200`. Paper only accepts the
instruction for a long option position on its expiration day. Raw Paper and
mock requests have succeeded, but the corrected Paper exact scenario still
needs a clean account and verified cleanup before `optionDoNotExercise` can be
closed.

## Not Implemented

- websocket / stream APIs
- broker APIs
- FIX
- crypto / fixed-income trading surfaces
- application-owned strategy orchestration or provider fallback systems

## Environment Variables

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- `ALPACA_TRADE_BASE_URL`

See `docs/reference/alpaca-trade.md` and <https://docs.rs/alpaca-trade> for the full reference.
