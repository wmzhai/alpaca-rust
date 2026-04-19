# alpaca-time

`alpaca-time` provides New York time and US trading-calendar semantics for the
`alpaca-rust` workspace.

Core modules:

- `clock`
- `calendar`
- `session`
- `expiration`
- `range`
- `display`

Use this crate when you need:

- canonical `YYYY-MM-DD` and `YYYY-MM-DD HH:MM:SS` parsing and formatting
- RFC3339 UTC to New York session/date normalization
- trading-day and market-session checks
- expiration-date math shared across the Rust workspace

This crate intentionally does not include:

- Alpaca HTTP clients
- option contracts, pricing, or payoff math
- cache orchestration or application scheduling

An optional workspace TypeScript companion exists under `packages/alpaca-time`,
but the published system surface is the Rust crate.

See `docs/reference/alpaca-time.md` and <https://docs.rs/alpaca-time> for the
full reference.
