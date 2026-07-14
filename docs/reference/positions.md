# Positions

`alpaca-trade::Client::positions()` exposes open-position routes.

## Implemented Methods

- `list`
- `get`
- `close_all`
- `close`
- `exercise`
- `do_not_exercise`

All six adopted position operations have public methods and mock routes. At the
Trading API `2.0.1` checkpoint, list, get, close-all, close by symbol or asset
ID, and exercise are closed against Paper and the standalone mock HTTP service.
Do-not-exercise is the only pending position operation.

## Typical Request

```rust
use rust_decimal::Decimal;
use alpaca_trade::{Client, positions};

let client = Client::from_env()?;
let close_order = client
    .positions()
    .close(
        "AAPL",
        positions::ClosePositionRequest {
            qty: Some(Decimal::ONE),
            ..positions::ClosePositionRequest::default()
        },
    )
    .await?;
# let _ = close_order;
# Ok::<(), alpaca_trade::Error>(())
```

## Request Notes

- symbol-or-id path values reject empty strings, path separators, and surrounding whitespace
- partial close requests preserve the official `qty` / `percentage` contract
- `qty` and `percentage` are mutually exclusive and positive; percentage is at most 100 with at most nine decimal places
- close by symbol returns the shared `Order` response with strict `200`
- close-all returns the canonical per-position multi-status response with strict `207`
- exercise strictly requires status `200`; `ExerciseAccepted.details` is `None` for the canonical empty body or contains typed `qty_exercised` and `qty_remaining` values for the JSON body observed from Paper
- do-not-exercise strictly requires an empty `200`; Paper accepts the instruction only for a long option position on its expiration day
- position models include canonical intraday P/L and optional quantity, swap-rate, and USD projections observed in the latest contract

## Important Scope Note

The positions resource exposes `exercise` and `do_not_exercise` because they are
part of the adopted mirror layer. Exercise is closed with a strict `200` status
and explicit support for both the canonical empty body and the typed JSON body
observed from Paper. Raw do-not-exercise requests have succeeded against Paper
and the standalone mock, but its corrected Paper exact scenario still needs a
clean account and verified cleanup. It therefore remains pending rather than
claiming `2.0.1` closure. The broader trading scope still excludes non-adopted
API families such as broker, FIX, streaming, and crypto surfaces.
