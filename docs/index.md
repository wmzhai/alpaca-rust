# alpaca-rust

`alpaca-rust` is the public documentation home for the workspace that publishes:

- `alpaca-core`
- `alpaca-http`
- `alpaca-data`
- `alpaca-trade`
- `alpaca-mock`

The workspace focuses on Alpaca HTTP APIs only. The current public release line excludes WebSocket, crypto, Broker API, FIX, and third-party provider clients.

Primary application entry points:

- `alpaca-data` for market data
- `alpaca-trade` for trading resources

Supporting crates:

- `alpaca-core` for shared primitives
- `alpaca-http` for transport behavior
- `alpaca-mock` for executable mock-server flows

Maintainer: Weiming Zhai <wmzhai@gmail.com>
