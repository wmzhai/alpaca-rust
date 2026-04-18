# Mock Server

`alpaca-mock` is the public mock-server executable for `alpaca-rust`.

## Run It

From an installed binary:

```bash
alpaca-mock
```

From the workspace:

```bash
cargo run -p alpaca-mock
```

Install it as a local user service on macOS or Ubuntu:

```bash
./scripts/install-alpaca-mock-service.sh
```

The service installer:

- builds `alpaca-mock` in release mode
- reads the root `.env`
- registers and starts a user service
- verifies `GET /health` before reporting success

Default listen address:

- `127.0.0.1:3847`

Override it with:

- `ALPACA_MOCK_LISTEN_ADDR`

On macOS the installer writes a launchd plist under `~/Library/LaunchAgents`.
On Ubuntu it writes a user systemd unit under `~/.config/systemd/user`.

## Authentication

Trading routes require Alpaca-style auth headers:

- `APCA-API-KEY-ID`
- `APCA-API-SECRET-KEY`

The mock server uses the API key to isolate per-account mock state.

## Public Routes

Unauthenticated:

- `GET /health`
- `GET /admin/state`
- `POST /admin/reset`
- `POST /admin/faults/http`

Authenticated trading routes:

- `GET /v2/account`
- `GET /v2/account/activities`
- `GET /v2/account/activities/{activity_type}`
- `GET|POST|DELETE /v2/orders`
- `GET|PATCH|DELETE /v2/orders/{order_id}`
- `GET /v2/orders:by_client_order_id`
- `GET|DELETE /v2/positions`
- `GET|DELETE /v2/positions/{symbol_or_asset_id}`
- `POST /v2/positions/{symbol_or_contract_id}/exercise`
- `POST /v2/positions/{symbol_or_contract_id}/do-not-exercise`

## Admin Endpoints

### `GET /admin/state`

Returns the current mock-state summary.

### `POST /admin/reset`

Clears the current mock state and fault injections.

### `POST /admin/faults/http`

Injects a transport-like HTTP fault for authenticated trading routes.
The injected fault is one-shot: it is consumed by the next authenticated trading request and then cleared.

Example:

```json
{
  "status": 503,
  "message": "temporary outage"
}
```

## Fill Behavior

The mock server keeps fill rules intentionally narrow and deterministic:

- stock and single-option market orders fill at mid price
- stock and single-option limit orders fill at mid price once the submitted limit reaches that mid price
- multi-leg market and limit orders use the composite mid price across all legs
- when a multi-leg limit reaches the composite mid price, the fill price is still that composite mid

## Scope

The current mock server is intentionally focused on the trade mainline:

- account
- orders
- positions
- activities

It is not a generic Alpaca emulator or a replacement for live API verification.

## Market Data Dependency

When order or position behavior needs live market prices, the mock server uses the `alpaca-data` client and real market-data credentials instead of invented price fixtures.
