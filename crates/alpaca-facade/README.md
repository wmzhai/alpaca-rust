# alpaca-facade

`alpaca-facade` provides high-level convenience facades built on top of the
lower-level `alpaca-rust` crates.

Current public surface:

- `AlpacaData` for cache-first raw market-data access plus option enrichment
- `AlpacaDataConfig` for facade-level pricing assumptions such as dividend yield
- `OptionChainRequest` for reusable option-chain filters
- bridge helpers such as `map_snapshot`, `map_live_snapshots`, and
  `resolve_positions_from_optionstrat_url`

Use this crate when you want:

- a reusable `alpaca-data` + `alpaca-option` + `alpaca-time` composition layer
- raw market-data caching without rebuilding the adapter stack yourself
- option-chain and enriched option-snapshot convenience helpers

This crate intentionally does not include:

- application singletons
- environment bootstrapping or config-file loading
- strategy orchestration or provider fallback logic

See `docs/reference/alpaca-facade.md` and <https://docs.rs/alpaca-facade> for
the full reference.
