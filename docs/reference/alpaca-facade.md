# alpaca-facade

`alpaca-facade` is the high-level convenience layer that composes the lower
`alpaca-rust` crates into application-friendly adapters.

## Main Types and Helpers

- `AlpacaData`
- `AlpacaDataConfig`
- `CacheStats`
- `OptionPricingReference`
- `ResolvedOptionStratPositions`
- `latest_close_prices(...)`
- `map_snapshot(...)`
- `map_snapshot_with_pricing_reference(...)`
- `map_snapshots(...)`
- `map_snapshots_with_pricing_references(...)`
- `map_live_snapshots(...)`
- `pricing_references_for_snapshots(...)`
- `resolve_positions_from_optionstrat_url(...)`

## Typical Uses

- Reuse `alpaca-data::cache::CachedClient` behind a richer option-aware facade
- Enrich Alpaca option snapshots into `alpaca-option` core models
- Repair missing or invalid provider IV and Greeks with a session-aware pricing
  reference
- Use realtime stock snapshots during regular session and recent daily-bar
  closes outside regular session
- Keep application-specific singleton or scheduling logic outside the shared crate

For option chains, call `alpaca-data` directly through
`client.options().chain_all(...)`, then map snapshots with
`map_live_snapshots(...)` if enriched `alpaca-option` models are needed.

## Not Included

- environment bootstrapping
- application singletons
- application-level provider failover orchestration
- strategy logic or order workflows

## Related Documents

- [alpaca-data](./alpaca-data.md)
- [alpaca-option](./alpaca-option.md)
- [alpaca-time](./alpaca-time.md)
- [alpaca-facade spec](https://github.com/wmzhai/alpaca-rust/tree/main/spec/alpaca-facade)
- [docs.rs/alpaca-facade](https://docs.rs/alpaca-facade)
