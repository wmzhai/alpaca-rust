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

Default listen address:

- `127.0.0.1:3847`

Override it with:

- `ALPACA_MOCK_LISTEN_ADDR`

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

Example:

```json
{
  "status": 503,
  "message": "temporary outage"
}
```

## Scope

The current mock server is intentionally focused on the trade mainline:

- account
- orders
- positions
- activities

It is not a generic Alpaca emulator or a replacement for live API verification.

## Market Data Dependency

When order or position behavior needs live market prices, the mock server uses the `alpaca-data` client and real market-data credentials instead of invented price fixtures.
