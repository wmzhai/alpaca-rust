# alpaca-option

`alpaca-option` is the provider-neutral option core in the `alpaca-rust`
workspace.

## Main Modules

- `contract`
- `display`
- `snapshot`
- `analysis`
- `pricing`
- `payoff`
- `probability`
- `execution_quote`
- `url`

## Typical Uses

- Parse and format OCC option contracts
- Work with canonical option snapshots, positions, chains, and execution quotes
- Compute pricing, Greeks, payoff, break-even points, and probability helpers
- Build and parse OptionStrat-compatible URLs and leg fragments

## Optional Companion

An optional workspace TypeScript companion exists under `packages/alpaca-option`.
It is a plus feature, not the primary published system surface.

The TypeScript package exports the following namespaces from `@alpaca/option`:

- `analysis`, `chain`, `contract`, `display`, `executionQuote`, `expirationSelection`
- `math` and sub-exports (`american`, `bachelier`, `barrier`, `black76`, `geometricAsian`)
- `numeric`, `optionStrategy`, `payoff`, `pricing`, `probability`, `snapshot`, `url`
- `OptionStrategy` class
- `OptionError`

Package metadata:

- `private: true`
- `@alpaca/option` (`1.10.4`)
- extra exports: `./math/american`, `./math/bachelier`, `./math/barrier`,
  `./math/black76`, `./math/geometric-asian`, `./option-strategy`.
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
