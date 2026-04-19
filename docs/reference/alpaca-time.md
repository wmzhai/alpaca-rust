# alpaca-time

`alpaca-time` is the workspace crate for New York time, US trading-calendar, and
expiration-date semantics shared across the Rust SDK surface.

## Main Modules

- `clock`
- `calendar`
- `session`
- `expiration`
- `range`
- `display`

## Typical Uses

- Normalize RFC3339 UTC timestamps into canonical New York timestamps
- Compare date-only and timestamp values without inventing intraday ordering
- Determine trading days, market sessions, week ranges, and expiration windows

## Optional Companion

An optional workspace TypeScript companion exists under `packages/alpaca-time`.
It is a plus feature, not the primary published system surface.

## Not Included

- Alpaca HTTP transport or credentials
- option contracts, payoff, or pricing math
- cache orchestration or background scheduling

## Related Documents

- [Project Structure](../project-structure.md)
- [Testing Guide](../testing.md)
- [alpaca-time spec](https://github.com/wmzhai/alpaca-rust/tree/master/spec/alpaca-time)
- [docs.rs/alpaca-time](https://docs.rs/alpaca-time)
