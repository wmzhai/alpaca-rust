# alpaca-rust

`alpaca-rust` is the public documentation home for the Rust workspace that
publishes:

- `alpaca-core`
- `alpaca-rest-http`
- `alpaca-data`
- `alpaca-trade`
- `alpaca-mock`
- `alpaca-time`
- `alpaca-option`
- `alpaca-facade`

The public Rust surface is split into three layers:

- Foundation SDK: direct Alpaca HTTP clients plus shared transport
- Semantic core: reusable market-time and option-domain semantics
- Convenience facade: higher-level composition on top of the lower layers

Primary application entry points:

- `alpaca-data`
- `alpaca-trade`
- `alpaca-time`
- `alpaca-option`
- `alpaca-facade`

Optional TypeScript companions exist inside the workspace, but they are plus
features rather than the primary published system surface.

Maintainer: Weiming Zhai (wmzhai@gmail.com)

## Documentation Map

- [Getting Started](./getting-started.md)
- [Installation](./installation.md)
- [Authentication](./authentication.md)
- [Project Structure](./project-structure.md)
- [Reference Index](./reference/index.md)
- [Mock Server](./mock-server.md)
- [Testing Guide](./testing.md)
- [Troubleshooting](./troubleshooting.md)
- [Release Checklist](./release-checklist.md)

## Coverage Snapshot

- `alpaca-data` covers the adopted Alpaca Market Data HTTP scope for stocks, options, news, and corporate actions
- `alpaca-trade` covers the adopted Alpaca Trading HTTP scope for account, activities, assets, calendar/clock, options contracts, orders, portfolio history, positions, and watchlists
- `alpaca-time`, `alpaca-option`, and `alpaca-facade` provide the shared semantic and convenience layers used above those HTTP crates

## Explicitly Out Of Scope

The current published Rust release line does not implement:

- market-data crypto, forex, fixed income, logos, or screener APIs
- trading crypto, fixed income, broker, FIX, websocket, or stream APIs
- strategy orchestration, provider fallback systems, or application singletons
