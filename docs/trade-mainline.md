# Trade Mainline

`alpaca-rust` treats the trading mainline as the resource chain that has to stay coherent across account state, order state, position state, observed fill activities, and the mock HTTP service.

## Covered resources

- `account`
- `orders`
- `positions`
- `activities`
- `alpaca-mock`

## Validation Matrix

The only Trading test lane is
`crates/alpaca-trade/tests/trading_contract_api.rs`. Its 28 scenarios use the
public client over real HTTP and run against either:

- canonical Paper, selected with `T127_TRADING_TARGET=paper`; or
- a separately running loopback mock service, selected with
  `T127_TRADING_TARGET=mock`.

Both targets use the same scenario code and contract assertions. Missing
credentials or target configuration fails immediately, and the test suite does
not contain a local fallback, in-process router, fixture, or algorithm-only
lane.

Run the selected target serially:

```bash
cargo test -p alpaca-trade --test trading_contract_api -- --nocapture --test-threads=1
```

## Activity contract note

The public activities surface intentionally mirrors the observed official paper contract. At the moment the mock server only exposes official-style `FILL` activity entries on `/v2/account/activities`.

Synthetic order lifecycle markers such as `NEW`, `REPLACED`, or `CANCELED` remain internal mock state only. They are not exported until an official response shape is captured and validated against the real paper API.

## Example

The mock lifecycle example can be run with live market data backing:

```bash
cargo run -p alpaca-trade --example mainline_mock_lifecycle
```

This example requires `ALPACA_DATA_API_KEY` and `ALPACA_DATA_SECRET_KEY`, because mock fills are still priced from real Alpaca market data.
It is a runnable demonstration, not part of the Cargo test suite or its
contract evidence.
