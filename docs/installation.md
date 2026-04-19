# Installation

## Library Crates

Add the Rust crate that matches the layer you want to use.

Market data:

```toml
[dependencies]
alpaca-data = "0.24.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Trading:

```toml
[dependencies]
alpaca-trade = "0.24.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Semantic and facade crates are also published:

- `alpaca-time`
- `alpaca-option`
- `alpaca-facade`

Foundation crates are also published:

- `alpaca-core`
- `alpaca-rest-http`

Most users should start from `alpaca-data`, `alpaca-trade`, `alpaca-time`,
`alpaca-option`, or `alpaca-facade` instead of depending on the low-level
foundation crates directly.

## Binary Crate

Install the mock server:

```bash
cargo install alpaca-mock
```

Run from the workspace:

```bash
cargo run -p alpaca-mock
```

Install and start it as a local user service on macOS or Ubuntu:

```bash
./scripts/install-alpaca-mock-service.sh
```

The service installer builds the release binary, registers the user service,
starts it, and checks `GET /health`.

## Credentials

Market data:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

Trading:

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`

Mock server binding:

- `ALPACA_MOCK_LISTEN_ADDR`

## Versioning Note

The workspace publishes multiple Rust crates that move together. If you depend
on more than one `alpaca-*` crate directly, keep their versions aligned.

Optional TypeScript companions exist inside the repo under `packages/`, but they
are not the primary published system surface for release planning.
