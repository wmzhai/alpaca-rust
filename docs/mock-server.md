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
- `POST /admin/market-data/stocks/{symbol}`

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
- `GET /v2/stocks/{symbol}/snapshot`

## Admin Endpoints

### `GET /admin/state`

Returns the current mock-state summary.

### `POST /admin/reset`

Clears the current mock state, runtime stock-price overrides, and fault injections.

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

### `POST /admin/market-data/stocks/{symbol}`

Sets a deterministic runtime price for a stock symbol. The price is rounded to two decimal places and exposed as the bid, ask, and latest trade through the authenticated `GET /v2/stocks/{symbol}/snapshot` route.

Existing open simple equity limit orders made marketable by the new price are filled exactly once. The response contains `symbol`, the normalized Decimal-string `price`, and sorted `filled_order_ids`. This control is intended only for deterministic local integration scenarios; it is not a general-purpose market-data generator.

## Fill Behavior

The mock server keeps fill rules intentionally narrow and deterministic:

- stock and single-option market orders fill at mid price
- stock and single-option limit orders fill at mid price once the submitted limit reaches that mid price
- multi-leg market and limit orders use the composite mid price across all legs
- when a multi-leg limit reaches the composite mid price, the fill price is still that composite mid

## Resting Limit Poller

When live market-data credentials are available, the executable starts one
background poller. Its first cycle runs 10 seconds after startup and subsequent
cycles run every 10 seconds. A slow cycle is completed serially; missed ticks
are skipped instead of overlapping requests.

The poller handles only top-level orders that are still `New`, have no filled
quantity, use type `limit`, and use time in force `day` or `gtc`:

- simple US equity orders
- simple US option orders
- option `mleg` orders

Stock symbols and option contracts are normalized and de-duplicated, then read
through independent logical snapshot batches. Options use the client batch and
pagination path. Runtime market-data overrides take precedence and live
snapshots are never stored as overrides. If either batch fails, a symbol is
missing, or a complete two-sided quote is unavailable, affected orders remain
unchanged and are retried on the next cycle; the other asset class can still be
processed.

Simple orders continue to use quote mid prices. An `mleg` order requires quotes
for every leg and uses the existing signed composite mid. A successful fill
updates the parent and nested legs, cash, positions, execution facts, and one
FILL activity exactly once. Orders are rechecked after market-data I/O so a
concurrent cancel, replace, reset, admin fill, or other state transition wins
without duplicate accounting.

The poller does not process bracket, OCO, OTO, stop, stop-limit,
trailing-stop, IOC, FOK, GTD, OPG, CLS, partial-fill, or advanced child-order
lifecycles. Without a live market-data bridge, the HTTP server starts normally
and no poller is created.

## Scope

The current mock server is intentionally focused on the trade mainline:

- account
- orders
- positions
- activities
- deterministic single-stock snapshots for integration scenarios

It is not a generic Alpaca emulator or a replacement for live API verification.

## Market Data Dependency

When no deterministic runtime override exists, order or position behavior that needs live market prices uses the `alpaca-data` client and real market-data credentials. Runtime overrides are limited to explicitly controlled local integration scenarios.
