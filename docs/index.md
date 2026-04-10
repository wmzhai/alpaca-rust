# alpaca-rust

`alpaca-rust` is the public documentation home for the workspace that publishes:

- `alpaca-core`
- `alpaca-rest-http`
- `alpaca-data`
- `alpaca-trade`
- `alpaca-mock`

The workspace focuses on Alpaca HTTP APIs only. The current public release line excludes WebSocket, crypto, Broker API, FIX, and third-party provider clients.

Primary application entry points:

- `alpaca-data` for market data
- `alpaca-trade` for trading resources

Supporting crates:

- `alpaca-core` for shared primitives
- `alpaca-rest-http` for transport behavior
- `alpaca-mock` for executable mock-server flows

Maintainer: Weiming Zhai <wmzhai@gmail.com>

## Documentation Map

- [Installation](./installation.md)
- [Getting Started](./getting-started.md)
- [Authentication](./authentication.md)
- [Project Structure](./project-structure.md)
- [Mock Server](./mock-server.md)
- [Testing Guide](./testing.md)
- [Troubleshooting](./troubleshooting.md)

## Coverage Snapshot

- `alpaca-data` adopts 28 market-data mirror operations across stocks, options, news, and corporate actions, with zero open mirror gaps in the current adopted scope
- `alpaca-trade` adopts 36 trading mirror operations across account, activities, assets, calendar/clock, options contracts, orders, portfolio history, positions, and watchlists, with zero open mirror gaps in the current adopted scope

## Explicitly Out Of Scope

The current public release line does not implement:

- market-data crypto, forex, fixed income, logos, or screener APIs
- trading crypto, fixed income, broker, FIX, websocket, or stream APIs
- order orchestration, strategy logic, provider fallback, caching, or application state management
