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

docs.rs:

- [docs.rs/alpaca-mock](https://docs.rs/alpaca-mock)

Current public mock focus:

- account
- orders
- positions
- activities

Not implemented:

- generic fake market-data generation
- a full broker or exchange simulator
