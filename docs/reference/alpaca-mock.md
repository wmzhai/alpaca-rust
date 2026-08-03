# alpaca-mock

Executable mock server for trade mainline and contract-style flows.

Commands:

```bash
cargo run -p alpaca-mock
cargo install alpaca-mock
```

Runtime environment:

- `ALPACA_MOCK_LISTEN_ADDR`
- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

Library helpers:

- `build_app_from_env`
- `build_app_with_state`
- `spawn_test_server`
- `spawn_test_server_with_state`
- `MockServerState::with_market_snapshot`

docs.rs:

- [docs.rs/alpaca-mock](https://docs.rs/alpaca-mock)

Current public mock focus:

- account
- account configurations
- portfolio history
- assets
- options contracts
- calendar and clock
- orders
- positions
- activities
- watchlists
- deterministic runtime stock snapshots

Behavior notes:

- stock and single-option marketable orders fill at mid price
- multi-leg marketable orders fill at composite mid price
- with a live market-data bridge, the executable polls resting `New` and unfilled `Day`/`GTC` limit orders every 10 seconds
- the poller supports simple stocks, simple options, and option `MLEG` orders; stock and option snapshots use independent logical batches and failures retry on the next cycle
- runtime overrides take precedence over live snapshots, and the poller rechecks each order before applying exactly-once fill effects
- without a live market-data bridge, the HTTP server runs without starting the poller
- account and watchlist identifiers use stable UUIDs and do not expose the API key
- watchlists keep ordered, per-account state and support the complete ID/name route family
- the Trading API `2.0.1` checkpoint has 37 operations closed with the same public network scenarios against Paper and this standalone HTTP service
- order cancel-all, order cancel by ID, and option exercise are closed; option do-not-exercise is the sole pending operation
- option exercise returns status `200` with typed `qty_exercised` and `qty_remaining` values, matching the body observed from Paper; the client also accepts the canonical empty `200`
- option do-not-exercise returns an empty `200`; raw Paper and mock requests have succeeded, but Paper restricts successful instructions to expiration-day long positions and the corrected exact Paper scenario still needs verified cleanup on a clean account
- `/admin/faults/http` injects a one-shot authenticated-route fault
- `POST /admin/market-data/stocks/{symbol}` sets a normalized runtime stock price and fills existing open simple equity limit orders that become marketable exactly once
- authenticated `GET /v2/stocks/{symbol}/snapshot` exposes the controlled price as bid, ask, and latest trade
- `/admin/reset` clears state, runtime stock-price overrides, and injected faults

Not implemented:

- general-purpose fake market-data generation; runtime stock controls exist only for deterministic integration scenarios
- bracket, OCO, OTO, stop, trailing-stop, partial-fill, and other advanced order lifecycles in the resting limit poller
- a full broker or exchange simulator
