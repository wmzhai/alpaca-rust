# Market Data API Coverage

This document is the public companion for `tools/api-coverage/market-data-api.json`.

## Scope

- Included: Alpaca Market Data HTTP API families used by the first US-equities-and-options-focused release
- Excluded: crypto, forex, fixed income, logos, screener, stream, websocket

## Adopted Families

- Stock
- Option
- News
- Corporate actions

## Implemented Mirror Coverage

Summary from `tools/api-coverage/market-data-api.json`:

- official total operations: `47`
- adopted-family total operations: `28`
- implemented mirror operations: `28`
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

Implemented adopted single-endpoint families:

- news list
- corporate actions list

## Explicitly Not Implemented

- crypto market data
- forex market data
- fixed income market data
- logos
- screener
- stream and websocket APIs

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Planned gaps and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
