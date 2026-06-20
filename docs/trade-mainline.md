# Trade Mainline

`alpaca-rust` treats the trading mainline as the resource chain that has to stay coherent across account state, order state, position state, observed fill activities, and the paper-backed mock server.

## Covered resources

- `account`
- `orders`
- `positions`
- `activities`
- `alpaca-mock`

## Validation Matrix

The workspace keeps both mainline lanes in `mainline_api.rs`:

- `trade_mainline_lifecycle_live_paper` exercises the paper trading API end to end with a real order open/close lifecycle.
- `trade_mainline_lifecycle_mock` exercises the same resource chain against `alpaca-mock`, while market prices still come from `alpaca-data`.
- `orders_api.rs` also keeps a deterministic mock contract lane that locks stock, single-option, and multi-leg create/replace fills at mid or composite-mid prices.

The recommended commands are:

```bash
cargo test -p alpaca-trade --test mainline_api trade_mainline_lifecycle_live_paper -- --nocapture
cargo test -p alpaca-trade --test mainline_api trade_mainline_lifecycle_mock -- --nocapture
cargo test -p alpaca-trade --tests -- --nocapture
cargo test -p alpaca-mock -- --nocapture
```

## Activity contract note

The public activities surface intentionally mirrors the observed official paper contract. At the moment the mock server only exposes official-style `FILL` activity entries on `/v2/account/activities`.

Synthetic order lifecycle markers such as `NEW`, `REPLACED`, or `CANCELED` remain internal mock state only. They are not exported until an official response shape is captured and validated against the real paper API.

## Example

The mock lane example can be run with live market data backing:

```bash
cargo run -p alpaca-trade --example mainline_mock_lifecycle
```

This example requires `ALPACA_DATA_API_KEY` and `ALPACA_DATA_SECRET_KEY`, because mock fills are still priced from real Alpaca market data.
