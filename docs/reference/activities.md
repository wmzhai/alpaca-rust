# Activities

`alpaca-trade::Client::activities()` exposes account activity history.

## Implemented Mirror Methods

- `list`
- `list_by_type`

## Convenience Methods

- `list_all`

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

Use `list_by_type` when the canonical path parameter is required:

```rust
use alpaca_trade::{Client, activities};

let client = Client::from_env()?;
let fills = client
    .activities()
    .list_by_type(
        "FILL",
        activities::ListByTypeRequest {
            page_size: Some(100),
            ..activities::ListByTypeRequest::default()
        },
    )
    .await?;
# let _ = fills;
# Ok::<(), alpaca_trade::Error>(())
```

## Request Notes

- `activity_types` must be non-empty when provided
- `activity_types` and `category` are mutually exclusive
- `page_size` must be between 1 and 100
- convenience methods follow pagination; they do not reshape the official activity payload

## Current Contract Notes

- the public type mirrors the observed official activity shape rather than imposing a closed enum over all future activity variants
- the category route and by-type route have distinct source and mock bindings
- canonical fields used by non-trade activity and the Paper-observed `description` and `execution_id` fields are typed; unknown response fields remain available through `extra`
- both canonical operations are closed against Paper and the standalone mock HTTP service
