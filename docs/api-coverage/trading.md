# Trading API Coverage

This document is the public companion for the machine-readable Trading API
coverage manifest.

## Canonical Baseline

- fetched: `2026-07-14`
- OpenAPI: `3.0.0`
- API version: `2.0.1`
- canonical operations: `68`
- canonical SHA-256: `17b9d12d5061e91e203eb91776c19107e572426d84e6d83969f48945791a6928`
- adopted operations: `38`
- Paper/mock validation closed: `37`
- validation pending: `1`

The canonical source is Alpaca's current Trading OpenAPI document. This
workspace does not preserve an older complete Trading specification, so the
coverage baseline compares the latest canonical contract with the current
source, mock route, and network-scenario bindings.

## Scope

- Included: non-crypto Alpaca Trading HTTP REST API
- Excluded: crypto, fixed income, stream, websocket, broker, FIX

## Adopted Tags And Resource Surfaces

- Accounts
- Account Activities
- Account Configurations
- Assets, including options contracts
- Calendar
- Clock
- Orders
- Portfolio History
- Positions
- Watchlists

## Adopted Operation Coverage

Summary from the coverage manifest:

- official total operations: `68`
- adopted-family total operations: `38`
- source-bound mirror operations: `38`
- mock route bindings: `38`
- closed Paper/mock network contracts: `37`
- pending Paper/mock network contracts: `1`

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

The sole pending operation is:

| operationId | Method and path | Why it is not closed |
| --- | --- | --- |
| `optionDoNotExercise` | `POST /v2/positions/{symbol_or_contract_id}/do-not-exercise` | Paper only accepts the instruction for a long option position on its expiration day. Raw Paper and mock requests returned the strict empty `200`, but the corrected Paper exact scenario still needs a clean account and verified cleanup. |

The method and mock route already exist. Raw endpoint success is not enough to
close the operation until the corrected exact scenario completes its Paper
state assertions and cleanup.

`optionExercise` is closed. It strictly requires status `200` while accepting
either the canonical empty body or the Paper-observed typed JSON object with
`qty_exercised` and `qty_remaining`. The standalone mock returns the observed
typed shape so the same public client path covers it.

## What Closed Means

Each closed operation has all of the following:

1. a successful request to the exact canonical Paper host using Paper credentials;
2. a post-change Paper request with status, non-secret request ID, and response-shape evidence;
3. a complete `alpaca-mock` HTTP route, request/response behavior, and state transition;
4. the same public `alpaca-trade` network scenario passing against Paper and a separately running mock process;
5. cleanup of any account configuration, order, position, or watchlist resource created by the scenario.

The network lane fails when the target, credentials, base URL, or observed HTTP
request is missing. It does not silently skip and does not call mock handlers or
state directly.

## Explicitly Not Implemented

- broker API
- FIX
- websocket and stream APIs
- crypto trading surfaces
- fixed-income trading surfaces
- locates
- IPO indication and IPO-only order-replace notional
- application-owned strategy orchestration or cross-provider execution engines

## Status

This is the current path-aware and contract-aware coverage-audit baseline for
`alpaca-rust`. It is an in-progress validation checkpoint, not a claim that all
38 adopted operations are closed.

- The machine-readable manifest exists.
- The audit entrypoint exists.
- Path-level adopted-family coverage checks are wired.
- Parameter-, body-, response-, requiredness-, nullability-, and array-level
  drift checks are wired against the current manifest contract snapshots.
- For closed rows, source bindings verify the client method, HTTP method/path,
  canonical operation label, strict transport helper/status, request contract,
  and public response model. Pending rows report their current source binding
  and the remaining validation reason without claiming closure.
- Mock bindings verify the registered route and handler for every row. Closed
  rows claim complete shared-scenario evidence; `optionDoNotExercise` has raw
  Paper and mock success but not complete exact-scenario cleanup evidence.
- Consumer bindings point to the exact network scenario and its closure status.
- Planned gaps, explicit family exclusions, explicit operation exclusions, and untracked official families are surfaced.
- Explicit exclusions are tracked in the manifest instead of being left as silent unknowns.
