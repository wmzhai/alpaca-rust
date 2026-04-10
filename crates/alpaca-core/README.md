# alpaca-core

`alpaca-core` provides low-level shared primitives for the `alpaca-rust` workspace.

It is intended for shared SDK internals and advanced integrations. Most users should start with `alpaca-data` or `alpaca-trade`.

Included building blocks:

- `Credentials`
- `BaseUrl`
- `QueryWriter`
- pagination helpers
- decimal and integer serde helpers
- lightweight validation utilities

What this crate does not include:

- Alpaca resource models such as `Order`, `Position`, `Account`, or `Activity`
- HTTP transport behavior
- resource clients such as `stocks()` or `orders()`
- application-level strategy, orchestration, caching, or state machines
