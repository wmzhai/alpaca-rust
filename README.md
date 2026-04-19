# alpaca-rust

`alpaca-rust` is a Rust workspace for Alpaca HTTP APIs.

The workspace is organized around five crates:

- `alpaca-data`: market data client for stocks, options, news, and corporate actions
- `alpaca-trade`: trading client for paper/live trading resources
- `alpaca-core`: shared primitives such as credentials, base URLs, query helpers, and serde helpers
- `alpaca-rest-http`: shared transport, retry, observer, and response metadata layer
- `alpaca-mock`: executable mock server for trade mainline and contract-style testing

Primary entry points for application code are `alpaca-data` and `alpaca-trade`.

Quick links:

- Repository: <https://github.com/wmzhai/alpaca-rust>
- GitHub Pages: <https://wmzhai.github.io/alpaca-rust/>
- `alpaca-data` docs.rs: <https://docs.rs/alpaca-data>
- `alpaca-trade` docs.rs: <https://docs.rs/alpaca-trade>

Maintainer: Weiming Zhai <wmzhai@gmail.com>

## What Is Included

- Market Data HTTP support for stocks, options, news, and corporate actions
- Trading HTTP support for account, account configurations, activities, assets, calendar, clock, options contracts, orders, portfolio history, positions, and watchlists
- Shared low-level crates for transport and shared primitives
- A mock server executable for trade-mainline and contract-style validation

## What Is Not Included

The current public release line does not implement:

- WebSocket or stream APIs
- Crypto
- Broker API
- FIX
- Third-party provider clients
- Strategy logic, order orchestration, cache layers, fallback providers, or stateful application workflows

Additional market-data families intentionally not covered in the current release line:

- forex
- fixed income
- logos
- screener

## Credentials

The workspace uses separate environment variables for market data and trading:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`

Optional overrides:

- `ALPACA_TRADE_BASE_URL`
- `ALPACA_MOCK_LISTEN_ADDR`

## Quick Start

Use `alpaca-data` for market data:

```toml
[dependencies]
alpaca-data = "0.24.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use alpaca_data::Client;

let client = Client::builder()
    .credentials_from_env()?
    .build()?;
# Ok::<(), alpaca_data::Error>(())
```

Use `alpaca-trade` for trading resources:

```toml
[dependencies]
alpaca-trade = "0.24.5"
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

Build `alpaca-mock` in release mode and run it as a local user service on macOS or Ubuntu:

```bash
./scripts/install-alpaca-mock-service.sh
```

The installer reads the root `.env`, registers the service, starts it, and verifies `GET /health`.
After it starts, point trading clients at:

```bash
ALPACA_TRADE_BASE_URL=http://127.0.0.1:3847
```

Mock execution semantics are intentionally simple:

- stock and single-option marketable orders fill at mid price
- multi-leg marketable orders fill at composite mid price
- limit orders become eligible as soon as the submitted limit reaches that mid or composite mid

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

## Crate Guide

### `alpaca-data`

Use this crate when you need the Alpaca Market Data HTTP API.

Implemented mirror coverage:

- stocks: bars, quotes, trades, latest bars/quotes/trades, snapshots, auctions, condition codes, exchange codes
- options: bars, trades, latest quotes/trades, snapshots, chain, condition codes, exchange codes
- news: list
- corporate actions: list

Thin convenience helpers currently included:

- `*_all` pagination aggregators for supported paginated endpoints
- stock snapshot convenience readers and stable ordering helpers
- provider-safe stock symbol normalization on stock request paths and batch query symbols

Not implemented in `alpaca-data`:

- crypto
- websocket
- stream APIs
- forex
- fixed income
- logos
- screener

### `alpaca-trade`

Use this crate when you need the Alpaca Trading HTTP API.

Implemented mirror coverage:

- account
- account configurations
- activities
- assets
- calendar and clock, including current v3 calendar/clock coverage
- options contracts
- orders
- portfolio history
- positions
- watchlists

Thin convenience helpers currently included:

- `list_all` / pagination collection for activities and options contracts where supported

Not implemented in `alpaca-trade`:

- websocket or stream APIs
- broker APIs
- FIX
- crypto and fixed income trading surfaces
- high-level order orchestration or application state machines

### `alpaca-core`

Use this crate only if you explicitly want shared low-level primitives. It is not intended to be the primary application entry point.

### `alpaca-rest-http`

Use this crate only if you explicitly want the low-level transport layer. Most SDK users should stay on `alpaca-data` or `alpaca-trade`.

### `alpaca-mock`

Use this crate when you need an executable mock server for integration tests or trade-mainline flows. It is a real public binary crate, but it remains purpose-built for SDK testing and development flows.
