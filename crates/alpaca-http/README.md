# alpaca-http

`alpaca-http` provides the shared transport layer for the `alpaca-rust` workspace.

It exposes request construction, retry policy, response metadata, observer hooks, and concurrency limiting. Most application code should use `alpaca-data` or `alpaca-trade` instead of depending on this crate directly.

Core pieces:

- `HttpClient`
- `HttpClientBuilder`
- `RetryConfig`
- `TransportObserver`
- `RequestParts`
- `ResponseMeta`
