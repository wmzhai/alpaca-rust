# Installation

## Library Crates

Add the crate you actually want to use.

Market data:

```toml
[dependencies]
alpaca-data = "0.24.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Trading:

```toml
[dependencies]
alpaca-trade = "0.24.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Low-level crates are also published:

- `alpaca-core`
- `alpaca-rest-http`

Most users should not start from those low-level crates.

## Binary Crate

Install the mock server:

```bash
cargo install alpaca-mock
```

Run from the workspace:

```bash
cargo run -p alpaca-mock
```

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

The workspace publishes multiple crates that move together. If you depend on more than one `alpaca-*` crate directly, keep their versions aligned.
