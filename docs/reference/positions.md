# Positions

`alpaca-trade::Client::positions()` exposes open-position routes.

## Implemented Methods

- `list`
- `get`
- `close_all`
- `close`
- `exercise`
- `do_not_exercise`

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

## Important Scope Note

The positions resource exposes `exercise` and `do_not_exercise` because they are part of the currently implemented mirror layer. The broader trading scope still excludes non-adopted API families such as broker, FIX, streaming, and crypto surfaces.
