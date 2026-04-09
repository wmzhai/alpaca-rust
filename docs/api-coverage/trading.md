# Trading API Coverage

This document is the public companion for `tools/api-coverage/trading-api.json`.

## Scope

- Included: non-crypto Alpaca Trading HTTP REST API
- Excluded: crypto, stream, websocket, broker, FIX

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Planned gaps and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
