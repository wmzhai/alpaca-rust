# alpaca-facade

`alpaca-facade` is the high-level convenience layer that composes the lower
`alpaca-rust` crates into application-friendly adapters.

## Main Types and Helpers

- `AlpacaData`
- `AlpacaDataConfig`
- `CacheStats`
- `OptionChainRequest`
- `ResolvedOptionStratPositions`
- `map_snapshot(...)`
- `map_snapshots(...)`
- `map_live_snapshots(...)`
- `fetch_chain(...)`
- `resolve_positions_from_optionstrat_url(...)`

## Typical Uses

- Reuse `alpaca-data::cache::CachedClient` behind a richer option-aware facade
- Enrich Alpaca option snapshots into `alpaca-option` core models
- Build and refresh filtered option chains with reusable request objects
- Keep application-specific singleton or scheduling logic outside the shared crate

## Not Included

- environment bootstrapping
- application singletons
- provider fallback orchestration
- strategy logic or order workflows

## Related Documents

- [alpaca-data](./alpaca-data.md)
- [alpaca-option](./alpaca-option.md)
- [alpaca-time](./alpaca-time.md)
- [alpaca-facade spec](https://github.com/wmzhai/alpaca-rust/tree/main/spec/alpaca-facade)
- [docs.rs/alpaca-facade](https://docs.rs/alpaca-facade)
