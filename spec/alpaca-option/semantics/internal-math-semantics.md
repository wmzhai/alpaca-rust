# alpaca-option Internal Math Semantics

This document defines the shared semantics for the internal `math/*` kernels.

It serves:

- mirrored Rust and TypeScript implementations
- fixture consistency
- public facades and higher-level wrappers built on the same numeric rules

Related structure definitions:

- `../models/internal-math-models.md`
- `../api/internal-math-kernels.md`

## Core Principles

- every input uses annualized decimal form by default
- `years` always means annualized time and is never derived from date strings inside the internal math layer
- the internal math layer prioritizes model-level numeric consistency over business-display convenience
- if a model output differs from a public facade expectation, the model fixture set remains authoritative

## `black76`

- `forward` is the futures-style forward price or underlying level
- `volatility` is lognormal annualized volatility
- `target_price` and the returned price both use discounted single-option prices
- `implied_volatility_from_price` only solves volatility and does not perform time conversion

Current implementation surface:

- `price`
- `greeks`
- `implied_volatility_from_price`

## `bachelier`

- `forward` and `strike` may be negative
- `normal_volatility` is normal annualized volatility in the same dimension as price
- `target_price` and the returned price both use discounted single-option prices
- `implied_volatility_from_price` returns `normal_volatility`

Current implementation surface:

- `price`
- `greeks`
- `implied_volatility_from_price`

## `american`

Shared input structures:

- `AmericanVanillaInput`
- `AmericanTreeInput`

### `tree_price`

- the current implementation is based on a CRR tree
- `use_richardson = true` enables Richardson extrapolation
- default step counts are promoted to sufficiently large even values to reduce numeric noise
- when the option is a call and `dividend_yield <= 0`, the implementation may degrade to the European price

### `barone_adesi_whaley_price`

- the current implementation does not support negative rates
- unsupported cases return `unsupported_math_input`

### `bjerksund_stensland_1993_price`

- explicitly fixed to the 1993 version
- it must not be silently replaced by the 2002 version without updating spec and fixtures

### `ju_quadratic_price`

- the current semantics are aligned to the QuantLib-compatible implementation and fixtures

## Frozen Semantics for Advanced Models

These models already have benchmark coverage in the latest dataset, so this document freezes their semantics and names. Both Rust and TypeScript currently implement the same named behavior.

### `american.discrete_dividend_price`

- the target model is American vanilla with fixed cash dividends
- `dividends` is an explicit cash schedule rather than a collapsed continuous `dividend_yield`
- `cash_dividend_model` only allows:
  - `spot`
  - `escrowed`
- `spot` and `escrowed` benchmarks must not be mixed; tests must compare against the declared model
- `expected.european_price` is only used to measure early-exercise premium and is not required as a separate public API
- the current benchmark source is QuantLib `FdBlackScholesVanillaEngine`, which is a high-quality numeric reference rather than an exact closed form
- to align point-by-point with the `option-test` dataset, both `years` and `dividend.time` are projected onto an `Actual365Fixed` day grid by `int(round(x * 365.0)) / 365` using Python banker’s rounding

### `barrier.price`

- the current scope is single-barrier, continuously monitored, BSM semantics with cash rebate
- `barrier_type` only allows:
  - `down_in`
  - `down_out`
  - `up_in`
  - `up_out`
- the benchmark owner is the analytic solution and QuantLib analytic barrier engine
- even if numeric methods are added later, the analytic benchmark remains the preferred regression truth
- to align with the `option-test` barrier dataset, `years` is also projected onto the `Actual365Fixed` day grid by `int(round(x * 365.0)) / 365` using Python banker’s rounding

### `geometric_asian.price`

- the current scope is BSM continuous geometric-average Asian
- `average_style` currently only allows `continuous`
- the benchmark is cross-validated between the closed-form formula and the QuantLib analytic engine
- this is a Tier A exact reference and should not be treated with loose engineering-approximation tolerance

## Current Support Matrix

| Module | Status | Price | Greeks | IV | Main limitations |
| --- | --- | --- | --- | --- | --- |
| `math.black76` | implemented | supported | supported | supported | uses lognormal volatility |
| `math.bachelier` | implemented | supported | supported | supported | IV returns `normal_volatility` |
| `math.american.tree` | implemented | supported | not provided | not provided | `steps` and `use_richardson` affect numeric precision |
| `math.american.barone_adesi_whaley` | implemented | supported | not provided | not provided | negative rates are not supported today |
| `math.american.bjerksund_stensland_1993` | implemented | supported | not provided | not provided | explicitly fixed to the 1993 version |
| `math.american.ju_quadratic` | implemented | supported | not provided | not provided | aligned to the QuantLib-compatible implementation |
| `math.american.discrete_dividend` | implemented | supported | not provided | not provided | built around cash schedules and `cash_dividend_model` |
| `math.barrier` | implemented | supported | not provided | not provided | current contract freezes continuous-monitoring single-barrier analytic semantics |
| `math.geometric_asian` | implemented | supported | not provided | not provided | current contract freezes continuous geometric-average analytic semantics |

## Greeks Unit Reminder

The internal `Greeks` structure does not add a blanket promise for:

- whether `vega` is per `1%` vol or per `1.00` vol
- whether `theta` is per day or per year
- whether `rho` is per `1bp` or per `1.00` rate

These units must be locked by each model’s fixture set rather than inferred from intuition.

## Error Semantics

### `invalid_math_input`

Used for:

- invalid `spot`, `forward`, or `strike`
- invalid `years`
- invalid `volatility` or `normal_volatility`
- invalid solver bounds or iteration counts
- tree-probability domain violations and similar input-level errors

### `root_not_bracketed`

Used for:

- IV solving when the target value is outside the current bracket

### `root_not_converged`

Used for:

- solver failure to converge within the allowed iteration count

### `unsupported_math_input`

Used for:

- model scenarios the current implementation explicitly does not support
- for example, the current BAW limitation on negative rates
- or runtime paths that have not yet been mirrored

## Testing and Regression Constraints

- Rust and TypeScript must both consume the same `fixtures/catalog.json` and share the internal-math fixtures in the `integrated` layer
- when internal-math changes require tolerance updates, fixture metadata must be updated before implementations are changed
- if a new model is added before a public facade exists, it must still add internal spec and fixtures first
- the current `option-test` research pack spans 7 datasets and 194 cases
- the 7-layer shared runner currently executes:
  1. `validated` / `reference`
  2. `stress` / `extreme_stability`
  3. `american_benchmark`
  4. `discrete_dividend_american`
  5. `exotics_analytic`

## Extension Rules

When adding models such as:

- arithmetic Asian
- Heston or SABR
- higher-precision PDE or tree models

follow the same two-layer pattern:

1. `../api/internal-math-kernels.md`: function signatures plus input and output contract
2. this file: model semantics, limitations, error codes, and unit notes
