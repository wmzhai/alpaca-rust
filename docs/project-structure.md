# Project Structure

The public workspace is split into five crates.

## `alpaca-core`

Shared primitives:

- credentials
- base URLs
- query writer
- pagination helpers
- decimal and integer serde helpers

Not here:

- resource clients
- Alpaca resource models
- transport execution

## `alpaca-rest-http`

Shared transport:

- request parts
- HTTP client wrapper
- retry configuration
- response and error metadata
- observer hooks
- concurrency limiting

Not here:

- resource-specific request builders
- market-data or trading models
- application state

## `alpaca-data`

Market Data SDK:

- stocks
- options
- news
- corporate actions

Not here:

- crypto
- websocket
- forex
- fixed income
- logos
- screener

## `alpaca-trade`

Trading SDK:

- account
- account configurations
- activities
- assets
- calendar
- clock
- options contracts
- orders
- portfolio history
- positions
- watchlists

Not here:

- broker API
- FIX
- trading websocket/stream APIs
- high-level order workflows

## `alpaca-mock`

Executable mock server with a thin library surface for tests and mock-state bootstrapping.

Not here:

- a fully generic Alpaca simulator
- invented fallback market data
