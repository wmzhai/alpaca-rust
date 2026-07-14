# alpaca-trade

`alpaca-trade` is the async Rust Trading HTTP client in the `alpaca-rust` workspace.

## Current Coverage

The current canonical baseline is Trading API `2.0.1`: 68 official operations,
38 adopted mirror operations, 37 closed Paper/mock network contracts, and 1
pending contract: option do-not-exercise.

All 38 public methods exist. Closure is stricter than method presence: the same
public client scenario must pass against Alpaca Paper and a separately running
`alpaca-mock` HTTP process, with the expected status, a non-empty request ID,
response-shape assertions, and cleanup.

### Implemented Resource Families

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

### Not Implemented

- broker APIs
- FIX
- crypto / fixed-income trading surfaces
- websocket / stream APIs
- application-owned strategy orchestration or provider fallback systems

### Optional Companion

There is a small workspace companion under `packages/alpaca-trade`
(`@alpaca/trade`) for TypeScript model sharing. It is a plus feature and is not
an additional published Rust crate or API surface.

`@alpaca/trade` currently only re-exports the generated `Execution` type used by
frontend consumers. The Rust `Execution` enum in
`crates/alpaca-trade/src/orders/execution.rs` is the source of truth and
retains the `ts-rs` export path for explicit binding generation. Cargo tests do
not generate TypeScript bindings.

## Client Entry

### Construction

- `Client::builder()`
- `Client::new(credentials)`
- `Client::from_env()`

### Common Builder Methods

- `credentials(...)`
- `api_key(...)`
- `secret_key(...)`
- `paper()`
- `live()`
- `base_url(...)`
- `base_url_str(...)`
- `credentials_from_env(...)`
- `credentials_from_env_names(...)`
- `base_url_from_env(...)`
- `timeout(...)`
- `reqwest_client(...)`
- `observer(...)`
- `retry_config(...)`
- `max_in_flight(...)`
- `build()`

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

## Account

- `account().get()`

## Account Configurations

- `account_configurations().get()`
- `account_configurations().update(...)`

## Activities

- `activities().list(...)`
- `activities().list_by_type(...)`
- `activities().list_all(...)`
- `activities().list_option_activity_records(...)`

### Convenience Helpers

- `Activity::{date, occurred_at, created_at, description, activity_sub_type, net_amount, per_share_amount, execution_id, sort_timestamp, qty_i32}`

## Assets

- `assets().list(...)`
- `assets().get(...)`

## Calendar / Clock

- `calendar().list(...)`
- `calendar().list_v3(...)`
- `clock().get()`
- `clock().get_v3(...)`

## Options Contracts

- `options_contracts().list(...)`
- `options_contracts().list_all(...)`
- `options_contracts().get(...)`

## Orders

- `orders().list(...)`
- `orders().create(...)`
- `orders().cancel_all()`
- `orders().get(...)`
- `orders().replace(...)`
- `orders().cancel(...)`
- `orders().get_by_client_order_id(...)`

All adopted order operations are closed for the `2.0.1` checkpoint, including
`cancel_all` and `cancel`.

### Convenience Helpers

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
- `SubmitOrderRequest`
- `SubmitOrderStyle`
- `SubmitOrderPolicy`
- `TransitionOrderPolicy`
- `TransitionResolution`
- `WaitFor`
- `ResolvedOrder`
- `OrderSide::as_str()`
- `OrderType::as_str()`
- `TimeInForce::as_str()`
- `PositionIntent::as_str()`
- `OrderClass::as_str()`
- `OrderStatus::as_str()`
- `Order::{qty_i32, filled_qty_i32}`

## Portfolio History

- `portfolio_history().get(...)`

## Positions

- `positions().list()`
- `positions().option_qty_map()`
- `positions().structure_quantity(...)`
- `positions().reconcile_signed_positions(...)`
- `positions().get(...)`
- `positions().close_all(...)`
- `positions().close(...)`
- `positions().exercise(...)`
- `positions().do_not_exercise(...)`

The list, get, close-all, close-by-symbol, and exercise operations are closed
for the `2.0.1` checkpoint. Exercise strictly requires status `200` and returns
`ExerciseAccepted`. Its optional `details` distinguishes the canonical empty
body from the Paper-observed JSON object containing `qty_exercised` and
`qty_remaining`.

Do-not-exercise strictly requires an empty `200`. Paper accepts it only for a
long option position on its expiration day. Raw Paper and mock requests have
succeeded, but the corrected Paper exact scenario still needs a clean account
and verified cleanup, so `optionDoNotExercise` remains the sole pending
operation.

## Watchlists

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

## Shared Helpers

- `pagination::collect_all(...)`

## Environment Variables

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- `ALPACA_TRADE_BASE_URL`

The default client targets Alpaca paper trading. Use `Client::builder().live()` or `base_url_str(...)` for a different endpoint.

## Related Documents

- [Account](./account.md)
- [Account Configurations](./account-configurations.md)
- [Activities](./activities.md)
- [Assets](./assets.md)
- [Calendar And Clock](./calendar-clock.md)
- [Options Contracts](./options-contracts.md)
- [Orders](./orders.md)
- [Portfolio History](./portfolio-history.md)
- [Positions](./positions.md)
- [Watchlists](./watchlists.md)
- [Trading API Coverage](../api-coverage/trading.md)
- [Trade Mainline](../trade-mainline.md)
- [docs.rs/alpaca-trade](https://docs.rs/alpaca-trade)
