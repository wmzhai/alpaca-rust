# alpaca-rust

`alpaca-rust` is a Rust workspace for Alpaca HTTP SDKs, market-time semantics,
provider-neutral option models, and high-level convenience facades.

The published Rust surface is organized into three layers:

- Foundation SDK: `alpaca-core`, `alpaca-rest-http`, `alpaca-data`, `alpaca-trade`, `alpaca-mock`
- Semantic core: `alpaca-time`, `alpaca-option`
- Convenience facade: `alpaca-facade`

Primary entry points depend on what you are building:

- use `alpaca-data` for direct market-data HTTP access
- use `alpaca-trade` for direct trading HTTP access
- use `alpaca-time` and `alpaca-option` for reusable domain semantics
- use `alpaca-facade` when you want the higher-level composition layer

Optional TypeScript companions exist under `packages/alpaca-time` and
`packages/alpaca-option`, but they are plus features inside the workspace, not
the recommended published system surface.

Quick links:

- Repository: <https://github.com/wmzhai/alpaca-rust>
- GitHub Pages: <https://wmzhai.github.io/alpaca-rust/>
- `alpaca-data` docs.rs: <https://docs.rs/alpaca-data>
- `alpaca-trade` docs.rs: <https://docs.rs/alpaca-trade>
- `alpaca-time` docs.rs: <https://docs.rs/alpaca-time>
- `alpaca-option` docs.rs: <https://docs.rs/alpaca-option>
- `alpaca-facade` docs.rs: <https://docs.rs/alpaca-facade>

Maintainer: Weiming Zhai <wmzhai@gmail.com>

## What Is Included

- Alpaca Market Data HTTP support for stocks, options, news, and corporate actions
- Alpaca Trading HTTP support for account, activities, assets, calendar, clock, options contracts, orders, portfolio history, positions, and watchlists
- Shared low-level transport, credentials, query, pagination, and serde primitives
- New York time and US trading-calendar semantics
- Provider-neutral option contracts, snapshots, pricing, payoff, and URL helpers
- A high-level facade crate that combines raw cache primitives with option enrichment helpers
- An executable mock server for trade-mainline and contract-style validation

## What Is Not Included

The current published Rust release line does not implement:

- WebSocket or stream APIs
- crypto, forex, fixed income, logos, or screener APIs
- Broker API or FIX
- third-party provider clients
- application singletons, strategy orchestration, or provider fallback systems

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
alpaca-data = "0.24.9"
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
alpaca-trade = "0.24.9"
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

Use `alpaca-facade` for the high-level composition layer:

```toml
[dependencies]
alpaca-facade = "0.24.9"
alpaca-data = "0.24.9"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use alpaca_data::cache::CachedClient;
use alpaca_data::Client;
use alpaca_facade::{AlpacaData, AlpacaDataConfig};

let raw = Client::builder()
    .credentials_from_env()?
    .build()?;
let facade = AlpacaData::with_raw(CachedClient::new(raw), AlpacaDataConfig::default());
# let _ = facade;
# Ok::<(), alpaca_data::Error>(())
```

Run the mock server:

```bash
cargo run -p alpaca-mock
```

Build `alpaca-mock` in release mode and run it as a local user service on macOS
or Ubuntu:

```bash
./scripts/install-alpaca-mock-service.sh
```

The installer reads the root `.env`, registers the service, starts it, and
verifies `GET /health`. After it starts, point trading clients at:

```bash
ALPACA_TRADE_BASE_URL=http://127.0.0.1:3847
```

## Crate Guide

### Foundation SDK

- `alpaca-core`: shared primitives such as credentials, base URLs, query helpers, and serde helpers
- `alpaca-rest-http`: shared transport, retry, observer, and response metadata layer
- `alpaca-data`: market data client for stocks, options, news, and corporate actions
- `alpaca-trade`: trading client for paper/live trading resources
- `alpaca-mock`: executable mock server for market-data-backed trade validation

### Semantic Core

- `alpaca-time`: New York time, trading calendar, expiration, and display semantics
- `alpaca-option`: provider-neutral option contracts, snapshots, chains, pricing, payoff, and URL helpers

### Convenience Facade

- `alpaca-facade`: high-level adapters that combine `alpaca-data`, `alpaca-time`, and `alpaca-option`

See `docs/`, the crate READMEs under `crates/`, and the published docs.rs pages
for the full reference.
