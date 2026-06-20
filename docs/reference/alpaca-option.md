# alpaca-option

`alpaca-option` is the provider-neutral option core in the `alpaca-rust`
workspace.

## Main Modules

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

## Typical Uses

- Parse and format OCC option contracts
- Work with canonical option snapshots, positions, chains, and execution quotes
- Compute pricing, Greeks, payoff, break-even points, probability helpers, and risk-free-rate defaults
- Analyze market structure, gamma exposure, liquidity models, and option-strategy state
- Build and parse OptionStrat-compatible URLs and leg fragments

## Optional Companion

An optional workspace TypeScript companion exists under `packages/alpaca-option`.
It is a plus feature, not the primary published system surface.

The TypeScript package exports the following namespaces from `@alpaca/option`:

- `analysis`, `chain`, `contract`, `display`, `executionQuote`, `expirationSelection`
- `math` and sub-exports (`american`, `bachelier`, `barrier`, `black76`, `geometricAsian`)
- `marketStructure`, `numeric`, `optionStrategy`, `payoff`, `pricing`, `probability`, `rate`, `snapshot`, `url`
- `OptionStrategy` class
- `OptionError`

Package metadata:

- `private: true`
- `@alpaca/option` (`1.10.4`)
- extra exports: `./math/american`, `./math/bachelier`, `./math/barrier`,
  `./math/black76`, `./math/geometric-asian`, `./market-structure`,
  `./option-strategy`, and `./rate`.
- dedicated TS tests cover public API boundary and fixture metadata expectations.

## Not Included

- Alpaca HTTP clients or credentials
- raw market-data transport and retry behavior
- application-specific singletons, caching, or strategy orchestration

## Related Documents

- [alpaca-facade](./alpaca-facade.md)
- [Project Structure](../project-structure.md)
- [alpaca-option spec](https://github.com/wmzhai/alpaca-rust/tree/main/spec/alpaca-option)
- [docs.rs/alpaca-option](https://docs.rs/alpaca-option)
