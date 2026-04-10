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
- the public API mirrors the official request shape instead of inventing higher-level workflow helpers

## Not Implemented Here

- order strategy orchestration
- smart defaults based on account state
- complex execution policy engines outside the official request contract
