# Trading API Coverage

This document is the public companion for `tools/api-coverage/trading-api.json`.

## Scope

- Included: non-crypto Alpaca Trading HTTP REST API
- Excluded: crypto, stream, websocket, broker, FIX
- Explicitly deferred from current coverage scope: `POST /v2/positions/{symbol_or_contract_id}/exercise`, `POST /v2/positions/{symbol_or_contract_id}/do-not-exercise`

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Planned gaps, explicit family exclusions, explicit operation exclusions, and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
