# Trading API Coverage

This document is the public companion for `tools/api-coverage/trading-api.json`.

## Scope

- Included: non-crypto Alpaca Trading HTTP REST API
- Excluded: crypto, fixed income, stream, websocket, broker, FIX

## Adopted Families

- Accounts
- Account Activities
- Account Configurations
- Assets
- Calendar
- Orders
- Portfolio History
- Positions
- Watchlists

## Implemented Mirror Coverage

Summary from `tools/api-coverage/trading-api.json`:

- official total operations: `64`
- adopted-family total operations: `36`
- implemented mirror operations: `36`
- open adopted-scope mirror gaps: `0`

Implemented resource groups:

- account: `GET /v2/account`
- account activities: list and list-by-type
- account configurations: get and update
- portfolio history: get
- assets: list and get
- calendar and clock: current v2 and adopted v3 routes
- options contracts: list, list-all convenience, get
- orders: list, create, cancel-all, get, replace, cancel, get-by-client-order-id
- positions: list, get, close-all, close, exercise, do-not-exercise
- watchlists: list, create, get/update/delete by id or name, add/delete assets

## Explicitly Not Implemented

- broker API
- FIX
- websocket and stream APIs
- crypto trading surfaces
- fixed-income trading surfaces
- high-level order orchestration or state-machine helpers

## Status

This is the current path-aware and contract-aware coverage-audit baseline for `alpaca-rust`.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-level and response-level drift checks are wired against the current manifest contract snapshots.
- Planned gaps, explicit family exclusions, explicit operation exclusions, and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
