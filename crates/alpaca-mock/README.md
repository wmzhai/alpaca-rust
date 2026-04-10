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

Runtime configuration:

- `ALPACA_MOCK_LISTEN_ADDR` defaults to `127.0.0.1:18080`
- market-data-backed flows require `ALPACA_DATA_API_KEY` and `ALPACA_DATA_SECRET_KEY`

The crate also exposes a thin library surface for test-server bootstrapping and mock state wiring.
