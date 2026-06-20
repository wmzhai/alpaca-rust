# alpaca-option

`alpaca-option` provides provider-neutral option semantics and math for the
`alpaca-rust` workspace.

Core modules:

- `analysis`
- `chain`
- `contract`
- `display`
- `execution_quote`
- `expiration_selection`
- `liquidity`
- `market_structure`
- `math`
- `numeric`
- `option_strategy`
- `payoff`
- `pricing`
- `probability`
- `rate`
- `snapshot`
- `url`

Use this crate when you need:

- OCC contract parsing and display helpers
- canonical option snapshots, positions, chains, and execution quotes
- Black-Scholes-style pricing, Greeks, payoff, probability, and risk-free-rate helpers
- market-structure analysis, liquidity models, and reusable option-strategy state
- OptionStrat-compatible URL and leg helpers

This crate intentionally does not include:

- Alpaca HTTP requests or credentials
- broker- or provider-specific transport behavior
- cache lifecycle, scheduling, or strategy orchestration

An optional workspace TypeScript companion exists under `packages/alpaca-option`,
but the published system surface is the Rust crate.

See `docs/reference/alpaca-option.md` and <https://docs.rs/alpaca-option> for
the full reference.
