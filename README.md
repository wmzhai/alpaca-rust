# alpaca-rust

`alpaca-rust` is a Rust workspace for Alpaca HTTP APIs.

The workspace is organized around five crates:

- `alpaca-data`: market data client for stocks, options, news, and corporate actions
- `alpaca-trade`: trading client for paper/live trading resources
- `alpaca-core`: shared primitives such as credentials, base URLs, query helpers, and serde helpers
- `alpaca-http`: shared transport, retry, observer, and response metadata layer
- `alpaca-mock`: executable mock server for trade mainline and contract-style testing

Primary entry points for application code are `alpaca-data` and `alpaca-trade`.

Quick links:

- Repository: <https://github.com/wmzhai/alpaca-rust>
- GitHub Pages: <https://wmzhai.github.io/alpaca-rust/>
- `alpaca-data` docs.rs: <https://docs.rs/alpaca-data>
- `alpaca-trade` docs.rs: <https://docs.rs/alpaca-trade>

Maintainer: Weiming Zhai <wmzhai@gmail.com>

## Credentials

The workspace uses separate environment variables for market data and trading:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`

Optional overrides:

- `ALPACA_DATA_BASE_URL`
- `ALPACA_TRADE_BASE_URL`
- `ALPACA_MOCK_LISTEN_ADDR`

## Quick Start

Use `alpaca-data` for market data:

```toml
[dependencies]
alpaca-data = "0.23.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use alpaca_data::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# Ok::<(), alpaca_data::Error>(())
```

Use `alpaca-trade` for trading resources:

```toml
[dependencies]
alpaca-trade = "0.23.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use alpaca_trade::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# Ok::<(), alpaca_trade::Error>(())
```

Run the mock server:

```bash
cargo run -p alpaca-mock
```

## Project Status

Current public scope:

- HTTP APIs only
- Market data for stocks and options, plus news and corporate actions
- Trading resources for account, assets, clock, calendar, orders, positions, activities, portfolio history, account configurations, options contracts, and watchlists
- Mock-server support for trade mainline validation

Explicitly out of scope for the current release line:

- WebSocket
- Crypto
- Broker API
- FIX
- Third-party provider clients
