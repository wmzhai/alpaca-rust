# alpaca-trade

Async trading client crate.

Resource accessors:

- `account()`
- `account_configurations()`
- `activities()`
- `assets()`
- `calendar()`
- `clock()`
- `options_contracts()`
- `orders()`
- `portfolio_history()`
- `positions()`
- `watchlists()`

Mirror coverage in the current release line:

- account
- account configurations
- account activities
- assets
- calendar and clock, including current adopted v3 clock/calendar routes
- options contracts
- orders
- portfolio history
- positions, including `exercise` and `do_not_exercise`
- watchlists

Convenience coverage:

- `activities().list_all(...)`
- `activities().list_by_type_all(...)`
- `options_contracts().list_all(...)`

Environment variables:

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- `ALPACA_TRADE_BASE_URL`

See also:

- [Trading API Coverage](../api-coverage/trading.md)
- [Trade Mainline](../trade-mainline.md)
- <https://docs.rs/alpaca-trade>

Not implemented:

- broker APIs
- FIX
- crypto and fixed-income trading surfaces
- websocket and stream APIs
- high-level order orchestration helpers
