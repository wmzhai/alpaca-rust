# News

`alpaca-data::Client::news()` exposes the Alpaca news listing endpoint.

## Implemented Methods

- `list`
- `list_all`

## Typical Request

```rust
use alpaca_data::{Client, news};

let client = Client::from_env()?;
let articles = client
    .news()
    .list(news::ListRequest {
        symbols: Some(vec!["AAPL".into(), "MSFT".into()]),
        start: Some("2026-04-01T00:00:00Z".into()),
        end: Some("2026-04-08T00:00:00Z".into()),
        limit: Some(20),
        ..news::ListRequest::default()
    })
    .await?;
# let _ = articles;
# Ok::<(), alpaca_data::Error>(())
```

## Request Notes

- `symbols` is optional, but if provided it must be non-empty
- `limit` follows the official range and is validated before the request is sent
- `list_all` follows server pagination until the full response is collected

## Not Implemented Here

- websocket or push-style news feeds
- custom relevance ranking or enrichment layers
