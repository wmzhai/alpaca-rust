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

When the live market-data bridge is configured, the executable starts one
10-second poller for resting limit orders. It supports only unfilled `New`
`Day`/`GTC` simple stock, simple option, and option `MLEG` orders. Stock and
option snapshots are fetched in independent logical batches; incomplete or
failed market-data reads leave affected orders unchanged for the next cycle.
Runtime overrides remain authoritative. Without live market-data credentials,
the HTTP server and deterministic admin controls still run, but the poller is
not started.

The crate also exposes a thin library surface for test-server bootstrapping and mock state wiring.

Current mock coverage is intentionally focused on the trade mainline:

- account
- orders
- positions
- activities

What `alpaca-mock` is not:

- a generic Alpaca simulator
- a replacement for live API verification
- a general-purpose fake market-data generator; runtime stock controls exist only for deterministic integration scenarios
- an engine for bracket, OCO, OTO, stop, trailing-stop, partial-fill, or other advanced order lifecycles
