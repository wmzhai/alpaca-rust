# alpaca-option Internal Math Kernels API

This document defines the mirrored API contract for the internal math kernels inside `alpaca-option`.

Notes:

- this is not the first-layer public API promise
- it is the repository-internal mirrored contract that Rust and TypeScript must both implement
- shared fixtures, numeric regressions, and higher-level facades treat this document as the source of truth

Stability layers:

- `public-api.md`: the canonical public API for business callers
- this document: the mirrored internal API for dual implementations, fixtures, review, and internal facades

## Shared Types and Input Structures

### `Greeks`

Notes:

- the internal math layer intentionally reuses the public `Greeks` structure
- the shared field set is fixed to `delta/gamma/vega/theta/rho`
- the model-specific units are still determined by the fixture set of each model rather than by the public Black-Scholes semantics
- the structure definition lives in `../models/internal-math-models.md`

Shared input structures:

- `Black76Input`
- `Black76IvInput`
- `BachelierInput`
- `BachelierIvInput`
- `AmericanVanillaInput`
- `AmericanTreeInput`
- `AmericanDiscreteDividendInput`
- `BarrierInput`
- `GeometricAsianInput`
- `SolverControl`

## `math.black76`

### `math.black76.price(input) -> number`

Where `input: Black76Input`.

### `math.black76.greeks(input) -> Greeks`

Where `input: Black76Input`.

### `math.black76.implied_volatility_from_price(input) -> number`

Where `input: Black76IvInput`.

## `math.bachelier`

### `math.bachelier.price(input) -> number`

Where `input: BachelierInput`.

### `math.bachelier.greeks(input) -> Greeks`

Where `input: BachelierInput`.

### `math.bachelier.implied_volatility_from_price(input) -> number`

Where `input: BachelierIvInput`.

Notes:

- `implied_volatility` here means `normal_volatility`
- the name stays aligned with the current implementation rather than introducing a separate `implied_normal_volatility` alias

## `math.american`

### `math.american.tree_price(input) -> number`

Where `input: AmericanTreeInput`.

### `math.american.barone_adesi_whaley_price(input) -> number`

Where `input: AmericanVanillaInput`.

### `math.american.bjerksund_stensland_1993_price(input) -> number`

Where `input: AmericanVanillaInput`.

### `math.american.ju_quadratic_price(input) -> number`

Where `input: AmericanVanillaInput`.

## Advanced Kernels

The following internal API names are already frozen in the spec to match the latest `option-test` dataset and the current dual implementations.

Current status:

- Rust: implemented
- TypeScript: implemented

### `math.american.discrete_dividend_price(input) -> number`

Where `input: AmericanDiscreteDividendInput`.

Notes:

- this is the canonical kernel for American vanilla options with fixed cash dividends
- `cash_dividend_model` is explicitly provided by the input rather than split into separate functions
- the numerical method stays replaceable inside the library and is intentionally not encoded into the function name

### `math.barrier.price(input) -> number`

Where `input: BarrierInput`.

Notes:

- the current contract is a canonical barrier kernel for single barriers, continuous monitoring, and BSM semantics
- if multiple methods such as `analytic`, `fd`, or `tree` need public exposure later, they should become separate, more granular functions

### `math.geometric_asian.price(input) -> number`

Where `input: GeometricAsianInput`.

Notes:

- the contract currently freezes only the continuous geometric-average Asian kernel
- discrete sampling, arithmetic averages, or MC and PDE approximations must be added as separate kernels instead of polluting this input

## Error Contract

The internal math layer currently standardizes these error codes:

- `invalid_math_input`
- `root_not_bracketed`
- `root_not_converged`
- `unsupported_math_input`

Notes:

- Rust and TypeScript should return the same error code for the same model and failure mode whenever possible
- new error codes should update this document before the dual implementations and tests are changed
- semantic meaning for each error code is defined in `../semantics/internal-math-semantics.md`

## Naming Mirror

- Rust: `snake_case`
- TypeScript: `camelCase`

Mappings:

- `implied_volatility_from_price` ↔ `impliedVolatilityFromPrice`
- `tree_price` ↔ `treePrice`
- `barone_adesi_whaley_price` ↔ `baroneAdesiWhaleyPrice`
- `bjerksund_stensland_1993_price` ↔ `bjerksundStensland1993Price`
- `ju_quadratic_price` ↔ `juQuadraticPrice`
- `discrete_dividend_price` ↔ `discreteDividendPrice`

## Out of Scope for This Contract

The following are intentionally not guaranteed by the internal math API contract today:

- a unified dispatcher that auto-selects models
- business-language facades built on top of the kernels
- provider data fetching or live enrichment
- arithmetic Asian, Heston, SABR, local-vol, Bermudan, or additional models not yet frozen here
