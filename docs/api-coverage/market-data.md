# Market Data API Coverage

This document is the public companion for `tools/api-coverage/market-data-api.json`.

## Scope

- Included: Alpaca Market Data HTTP API families used by the first US-equities-and-options-focused release
- Excluded: crypto, forex, fixed income, logos, screener, stream, websocket

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Planned gaps and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
