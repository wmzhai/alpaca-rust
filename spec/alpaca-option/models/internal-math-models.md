# alpaca-option Internal Math Models

This document defines the shared input and output structures used by the internal `math/*` kernels.

Notes:

- these structures are not the first-layer public models
- they are the mirrored model contract for the Rust and TypeScript internal math layers
- fixtures, internal facades, and precision-model integration follow this document

## Basic Literal Types

### `AnnualizedYears`

- `number`
- represents annualized time such as `30 / 365`

### `Rate`

- `number`
- decimal form, for example `0.045 = 4.5%`

### `Volatility`

- `number`
- lognormal annualized volatility

### `NormalVolatility`

- `number`
- normal annualized volatility

### `CashDividendModel`

```text
CashDividendModel = "spot" | "escrowed"
```

### `BarrierType`

```text
BarrierType = "down_in" | "down_out" | "up_in" | "up_out"
```

### `AverageStyle`

```text
AverageStyle = "continuous"
```

Notes:

- the current `option-test` dataset only freezes continuous geometric Asian semantics
- if discrete averaging is added later, it must use a distinct literal set rather than reusing this one

## Shared Output Structure

### `Greeks`

```text
Greeks = {
  delta: number,
  gamma: number,
  vega: number,
  theta: number,
  rho: number
}
```

Notes:

- the internal math layer and the public layer intentionally share this structure
- the field set is fixed to `delta/gamma/vega/theta/rho`
- units are locked by the fixtures and semantics of each model rather than by a single blanket rule

## Shared Schedule Structure

### `CashDividend`

```text
CashDividend = {
  time: AnnualizedYears,
  amount: number
}
```

Notes:

- `time` uses annualized time rather than date strings
- `amount` is a fixed cash dividend amount rather than a yield
- callers are responsible for sorting the schedule by time

## Shared Solver Controls

### `SolverControl`

```text
SolverControl = {
  lower_bound?: number,
  upper_bound?: number,
  tolerance?: number,
  max_iterations?: number
}
```

Notes:

- every optional field is solver tuning rather than business semantics
- higher-level facades should not expose these knobs to ordinary business callers unless the numeric need is explicit

## `black76`

### `Black76Input`

```text
Black76Input = {
  forward: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  volatility: Volatility,
  option_right: OptionRight
}
```

### `Black76IvInput`

```text
Black76IvInput = {
  target_price: number,
  forward: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  option_right: OptionRight,
  lower_bound?: number,
  upper_bound?: number,
  tolerance?: number,
  max_iterations?: number
}
```

## `bachelier`

### `BachelierInput`

```text
BachelierInput = {
  forward: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  normal_volatility: NormalVolatility,
  option_right: OptionRight
}
```

### `BachelierIvInput`

```text
BachelierIvInput = {
  target_price: number,
  forward: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  option_right: OptionRight,
  lower_bound?: number,
  upper_bound?: number,
  tolerance?: number,
  max_iterations?: number
}
```

Notes:

- the API name still remains `implied_volatility_from_price`
- the owner of the volatility field in this model is explicitly `normal_volatility`

## `american`

### `AmericanVanillaInput`

```text
AmericanVanillaInput = {
  spot: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  dividend_yield: Rate,
  volatility: Volatility,
  option_right: OptionRight
}
```

### `AmericanTreeInput`

```text
AmericanTreeInput = {
  spot: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  dividend_yield: Rate,
  volatility: Volatility,
  option_right: OptionRight,
  steps?: number,
  use_richardson?: boolean
}
```

Notes:

- `AmericanTreeInput` extends `AmericanVanillaInput`
- `steps` and `use_richardson` are numeric-method controls rather than product semantics

### `AmericanDiscreteDividendInput`

```text
AmericanDiscreteDividendInput = {
  spot: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  volatility: Volatility,
  option_right: OptionRight,
  cash_dividend_model: CashDividendModel,
  dividends: CashDividend[]
}
```

Notes:

- this structure only describes fixed cash dividends and does not mix in `dividend_yield`
- `cash_dividend_model` is currently limited to `spot` or `escrowed`
- an empty dividend schedule should not use this model

## `barrier`

### `BarrierInput`

```text
BarrierInput = {
  spot: number,
  strike: number,
  barrier: number,
  rebate: number,
  years: AnnualizedYears,
  rate: Rate,
  dividend_yield: Rate,
  volatility: Volatility,
  option_right: OptionRight,
  barrier_type: BarrierType
}
```

Notes:

- the current scope is single-barrier, continuously monitored, BSM semantics
- `rebate` is a cash rebate and may be `0`

## `geometric_asian`

### `GeometricAsianInput`

```text
GeometricAsianInput = {
  spot: number,
  strike: number,
  years: AnnualizedYears,
  rate: Rate,
  dividend_yield: Rate,
  volatility: Volatility,
  option_right: OptionRight,
  average_style: AverageStyle
}
```

Notes:

- `average_style` is currently frozen to `continuous`
- fixed-observation geometric averages should add a new input structure rather than reuse this semantics

## Naming Mirror

Field naming conventions:

- Rust: `snake_case`
- TypeScript: `camelCase`

Mappings:

- `option_right` ↔ `optionRight`
- `target_price` ↔ `targetPrice`
- `dividend_yield` ↔ `dividendYield`
- `normal_volatility` ↔ `normalVolatility`
- `cash_dividend_model` ↔ `cashDividendModel`
- `barrier_type` ↔ `barrierType`
- `average_style` ↔ `averageStyle`
- `lower_bound` ↔ `lowerBound`
- `upper_bound` ↔ `upperBound`
- `max_iterations` ↔ `maxIterations`
- `use_richardson` ↔ `useRichardson`

## Out of Scope for This Model Layer

The following are intentionally not frozen by the internal math models today:

- PDE grid-control parameters
- arithmetic Asian averaging schedules
- Heston, SABR, or local-vol parameter vectors
- multi-barrier, multi-asset, or Bermudan extension structures

New models should add their input structures here first.
