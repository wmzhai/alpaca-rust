# alpaca-mock

`alpaca-mock` is an executable mock server for `alpaca-rust` trade flows.

Install:

```bash
cargo install alpaca-mock
```

Run locally:

```bash
cargo run -p alpaca-mock
```

From the workspace repository root, install and start it as a local user service on macOS or Ubuntu:

```bash
./scripts/install-alpaca-mock-service.sh
```

Runtime configuration:

- `ALPACA_MOCK_LISTEN_ADDR` defaults to `127.0.0.1:3847`
- market-data-backed flows require `ALPACA_DATA_API_KEY` and `ALPACA_DATA_SECRET_KEY`

The crate also exposes a thin library surface for test-server bootstrapping and mock state wiring.

Current mock coverage is intentionally focused on the trade mainline:

- account
- orders
- positions
- activities

What `alpaca-mock` is not:

- a generic Alpaca simulator
- a replacement for live API verification
- a fake market-data generator with invented prices
