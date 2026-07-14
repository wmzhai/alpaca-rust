# Orders

`alpaca-trade::Client::orders()` exposes order lifecycle endpoints.

## Implemented Methods

- `list`
- `create`
- `cancel_all`
- `get`
- `replace`
- `cancel`
- `get_by_client_order_id`

All seven adopted order operations are closed against Paper and the standalone
mock HTTP service at the Trading API `2.0.1` checkpoint, including cancel-all
and cancel by order ID.

## Lifecycle Helpers

The mirror methods above stay close to Alpaca's official request and response
shapes. The resource client also exposes opt-in lifecycle helpers for callers
that need to wait for stable order state or recover from request uncertainty:

- `create_resolved`
- `get_effective`
- `wait_for`
- `cancel_resolved`
- `replace_resolved`
- `submit_with_policy`
- `submit_resolved`
- `close_option_legs`
- `recover_market_close`
- `transition_resolved`

Supporting public types include `WaitFor`, `ResolvedOrder`,
`ReplaceResolution`, `SubmitOrderRequest`, `SubmitOrderStyle`,
`SubmitOrderPolicy`, `TransitionOrderPolicy`, and `TransitionResolution`.

`SubmitOrderRequest` supports additive builders for caller-owned order identity
and explicit simple-order intent:

- `with_client_order_id(...)` applies to simple and multi-leg create requests.
- `with_position_intent(...)` applies an explicit `PositionIntent` to simple
  requests.

When a create request has a client order ID, `create_resolved` recovers an
ambiguous create by looking up that ID, validating the recovered order shape,
and waiting for the requested stable state. A recovered order must match the
submitted class, quantity, execution fields, legs, ratios, sides, and position
intents.

`TransitionOrderPolicy::Recreate` is the strict cancel-and-create mode. A
recreate request must include a stable client order ID so a response-loss retry
can adopt the same replacement. After cancellation, the transition recursively
checks the parent and nested child fill quantities; it creates the replacement
only when all fill evidence is zero. `TransitionOrderPolicy::Auto` keeps its
existing replace/recreate selection semantics.

## Mirror Contract Notes

- `ListRequest` supports typed CSV `asset_class`, `before_order_id`, and `after_order_id`; order-ID cursors are mutually exclusive and cannot be combined with the time window
- `GetRequest { nested: Some(true) }` explicitly requests multi-leg children; lifecycle helpers use it whenever the leg shape matters
- `Order::legs` contains non-recursive `OrderLeg` values, including the observed string-form `ratio_qty`
- create supports ordinary quantity orders, notional orders, typed advanced instructions, and 2–4-leg MLEG orders proven against Paper
- replace supports ordinary and TWAP advanced updates proven against Paper
- IPO indication and its IPO-only replace `notional` field are outside the adopted scope
- direct cancel by ID expects strict empty `204`; cancel-all expects strict `207` with the canonical per-order result array

## Typical Request

```rust
use rust_decimal::Decimal;
use alpaca_trade::{
    Client,
    orders::{CreateRequest, OrderSide, OrderType, TimeInForce},
};

let client = Client::from_env()?;
let order = client
    .orders()
    .create(CreateRequest {
        symbol: Some("AAPL".into()),
        qty: Some(Decimal::ONE),
        side: Some(OrderSide::Buy),
        r#type: Some(OrderType::Market),
        time_in_force: Some(TimeInForce::Day),
        ..CreateRequest::default()
    })
    .await?;
# let _ = order;
# Ok::<(), alpaca_trade::Error>(())
```

## Request Notes

- path identifiers are validated to reject empty strings and `/`
- symbol-like text fields reject empty or whitespace-only values
- direct mirror methods preserve the official request shape
- lifecycle helpers are explicit opt-in conveniences and do not replace the raw order endpoints
- strict recreate callers own the stable client order ID for each replacement generation
- Paper currently includes the anchor order for both order-ID cursor directions although the canonical specification describes the cursor as exclusive; the mock keeps canonical exclusive behavior
- Paper can return nested legs even for `nested=false`; callers that depend on legs must still request `nested=true`

## Not Implemented Here

- application-owned order strategy orchestration
- smart defaults based on account state
- cross-provider execution engines outside the Alpaca contract
