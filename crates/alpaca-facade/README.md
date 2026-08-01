# alpaca-facade

`alpaca-facade` provides high-level convenience facades built on top of the
lower-level `alpaca-rust` crates.

Current public surface:

- `AlpacaData` for cache-first raw market-data access plus option enrichment
- `AlpacaDataConfig` for facade-level pricing assumptions such as dividend yield
- bridge helpers such as `map_snapshot`, `map_live_snapshots`, and
  `resolve_positions_from_optionstrat_url`

Use this crate when you want:

- a reusable `alpaca-data` + `alpaca-option` + `alpaca-time` composition layer
- raw market-data caching without rebuilding the adapter stack yourself
- enriched option-snapshot convenience helpers

This crate intentionally does not include:

- application singletons
- environment bootstrapping or config-file loading
- strategy orchestration or provider fallback logic

## Enriched Option Cache Contract

`AlpacaData` serializes option watch, cache-miss enrichment, periodic rebuild, OptionStrat raw snapshot resolution, and cache clear operations through one lifecycle gate. The raw and enriched `RwLock` guards remain short-lived and are never held across provider I/O, while a completed clear cannot be undone by an older in-flight facade refresh.

Periodic option refresh returns raw provider and enrichment failures to the host. A failure keeps the prior enriched cache and does not advance its success timestamp. Successful full, partial, and empty reconciliation replaces the requested enriched state atomically, records omitted contracts as unavailable, and advances the success timestamp even when the provider returns no requested contracts.

See `docs/reference/alpaca-facade.md` and <https://docs.rs/alpaca-facade> for
the full reference.
