# Calendar And Clock

`alpaca-trade::Client::calendar()` and `alpaca-trade::Client::clock()` expose market-session timing data.

## Implemented Methods

- calendar: `list`, `list_v3`
- clock: `get`, `get_v3`

Canonical operations `LegacyCalendar`, `Calendar`, `LegacyClock`, and `Clock`
are closed against both Paper and the standalone mock HTTP service.

## Typical Request

```rust
use alpaca_trade::{Client, calendar};

let client = Client::from_env()?;
let days = client
    .calendar()
    .list(calendar::ListRequest {
        start: Some("2026-04-01".into()),
        end: Some("2026-04-30".into()),
        ..calendar::ListRequest::default()
    })
    .await?;
# let _ = days;
# Ok::<(), alpaca_trade::Error>(())
```

## Notes

- both current adopted v2 and v3 routes are documented in the coverage manifest
- v2 calendar supports the typed `TRADING` and `SETTLEMENT` date types
- v3 calendar accepts a typed market and UTC output-time request; the returned market metadata still reports the market's own `America/New_York` timezone
- v3 clock validates RFC 3339 input and compares instants rather than requiring the response to preserve the request's textual UTC offset
- timestamp and time-like fields stay in their official string forms in the public model layer

## Not Implemented Here

- market-holiday forecasting beyond official responses
- websocket time updates
