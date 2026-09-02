# Market Data API Coverage

This document is the public companion for `tools/api-coverage/market-data-api.json`.

## Scope

- Included: Alpaca Market Data HTTP API families used by the US-equities, options, and crypto-spot release
- Excluded or deferred: crypto perpetual futures, forex, fixed income, index, logos, screener, stream, websocket

## Adopted Families

- Stock
- Option
- Crypto
- News
- Corporate actions

## Implemented Mirror Coverage

Summary from `tools/api-coverage/market-data-api.json`:

- official total operations: `50`
- adopted-family total operations: `36`
- implemented mirror operations: `36`
- open adopted-scope mirror gaps: `0`

Implemented stock operations:

- historical bars, quotes, trades
- single-symbol historical bars, quotes, trades
- latest bars, quotes, trades
- snapshots and single-symbol snapshot
- auctions and single-symbol auctions
- condition codes and exchange codes

Implemented option operations:

- bars
- trades
- latest quotes
- latest trades
- snapshots
- chain
- condition codes
- exchange codes

Implemented crypto operations:

- historical bars, quotes, trades
- latest bars, quotes, trades
- latest orderbooks
- snapshots

Implemented adopted single-endpoint families:

- news list
- corporate actions list

## Explicitly Not Implemented

- crypto perpetual futures market data
- forex market data
- fixed income market data
- index data, deferred to `optworks#173` until the Paper entitlement can verify the real API
- logos
- screener
- stream and websocket APIs

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Conditional single-symbol route dispatch is checked inside each declared method's actual `single_symbol` branch.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Corporate Actions additionally checks the 15 action types plus required-field and nullability paths.
- Planned gaps and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
