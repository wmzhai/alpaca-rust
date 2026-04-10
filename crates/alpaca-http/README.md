# alpaca-rest-http

`alpaca-rest-http` provides the shared transport layer for the `alpaca-rust` workspace.

It exposes request construction, retry policy, response metadata, observer hooks, and concurrency limiting. Most application code should use `alpaca-data` or `alpaca-trade` instead of depending on this crate directly.

Core pieces:

- `HttpClient`
- `HttpClientBuilder`
- `RetryConfig`
- `TransportObserver`
- `RequestParts`
- `ResponseMeta`

What this crate is for:

- low-level HTTP client reuse across `alpaca-data`, `alpaca-trade`, and `alpaca-mock`
- retry, backoff, error metadata, and request observer wiring
- generic request execution against configurable base URLs

What this crate is not for:

- Alpaca resource semantics
- typed market-data or trading models
- cache or subscription systems
- application lifecycle management
