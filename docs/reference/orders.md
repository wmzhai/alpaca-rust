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

## Not Implemented Here

- application-owned order strategy orchestration
- smart defaults based on account state
- cross-provider execution engines outside the Alpaca contract
