# Project Structure

The public workspace is split into five crates.

## `alpaca-core`

Shared primitives:

- credentials
- base URLs
- query writer
- pagination helpers
- decimal and integer serde helpers

## `alpaca-http`

Shared transport:

- request parts
- HTTP client wrapper
- retry configuration
- response and error metadata
- observer hooks
- concurrency limiting

## `alpaca-data`

Market Data SDK:

- stocks
- options
- news
- corporate actions

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

## `alpaca-mock`

Executable mock server with a thin library surface for tests and mock-state bootstrapping.
