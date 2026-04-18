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
- orders
- positions
- activities

Behavior notes:

- stock and single-option marketable orders fill at mid price
- multi-leg marketable orders fill at composite mid price
- `/admin/faults/http` injects a one-shot authenticated-route fault
- `/admin/reset` clears both state and injected faults

Not implemented:

- generic fake market-data generation
- a full broker or exchange simulator
