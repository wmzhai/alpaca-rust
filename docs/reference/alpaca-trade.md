# alpaca-trade

`alpaca-trade` is the async Rust Trading HTTP client in the `alpaca-rust` workspace.

## Current Coverage

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
- high-level order orchestration helpers

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
- `activities().list_all(...)`

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

### Convenience Helpers

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
- `positions().get(...)`
- `positions().close_all(...)`
- `positions().close(...)`
- `positions().exercise(...)`
- `positions().do_not_exercise(...)`

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
