# Activities

`alpaca-trade::Client::activities()` exposes account activity history.

## Implemented Mirror Methods

- `list`
- `list_by_type`

## Convenience Methods

- `list_all`
- `list_by_type_all`

## Typical Request

```rust
use alpaca_trade::{Client, activities};

let client = Client::from_env()?;
let fills = client
    .activities()
    .list(activities::ListRequest {
        activity_types: Some(vec!["FILL".into()]),
        page_size: Some(100),
        ..activities::ListRequest::default()
    })
    .await?;
# let _ = fills;
# Ok::<(), alpaca_trade::Error>(())
```

## Request Notes

- `activity_types` must be non-empty when provided
- `page_size` must be greater than zero
- convenience methods follow pagination; they do not reshape the official activity payload

## Current Contract Notes

- the public type mirrors the observed official activity shape rather than imposing a closed enum over all future activity variants
- the mock server focuses on `FILL`-style trade activity output for mainline validation
