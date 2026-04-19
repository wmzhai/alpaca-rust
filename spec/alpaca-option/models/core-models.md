# alpaca-option Core Models

This document defines the canonical v1 core models for `alpaca-option`.

## Basic Literal Types

- `OptionRight`: `call | put`
- `OptionRightCode`: `C | P`
- `PositionSide`: `long | short`
- `OrderSide`: `buy | sell`
- `ExecutionAction`: `open | close`
- `PositionIntent`: `buy_to_open | sell_to_open | buy_to_close | sell_to_close`
- `MoneynessLabel`: `itm | atm | otm`
- `AssignmentRiskLevel`: `danger | critical | high | medium | low | safe`
- `NyDateString` and `NyTimestampString` are reused from `alpaca-time`

## Core Models

### `OptionContract`

```text
{
  underlying_symbol: string,
  expiration_date: NyDateString,
  strike: number,
  option_right: OptionRight,
  occ_symbol: string
}
```

Constraints:

- `underlying_symbol` uses OCC semantics and does not try to restore display symbols such as `BRKB -> BRK.B`
- `underlying_symbol` may retain OCC-adjusted suffix digits such as `TSLL1`
- `occ_symbol` is the canonical contract primary key
- `strike` is always a `number` in the core layer; the front end should not keep orbiting around `string -> parseFloat`

### `OptionQuote`

```text
{
  bid: number | null,
  ask: number | null,
  mark: number | null,
  last: number | null
}
```

Migration constraints:

- the legacy `snapshot.price` field converges to `quote.mark`
- the core model does not keep a second canonical top-level `price` field parallel to `quote.mark`
- legacy flat snapshots are absorbed through `execution_quote.quote`
- when a provider only exposes one side of the quote, the adapter or builder owns the degraded `mark` rule

### `Greeks`

```text
{
  delta: number,
  gamma: number,
  vega: number,
  theta: number,
  rho: number
}
```

### `OptionSnapshot`

```text
{
  as_of: NyTimestampString,
  contract: OptionContract,
  quote: OptionQuote,
  greeks: Greeks | null,
  implied_volatility: number | null,
  underlying_price: number | null
}
```

Migration constraints:

- legacy `timestamp` becomes `as_of`
- legacy `iv` becomes `implied_volatility`
- legacy flat `bid / ask / price` become `quote.bid / quote.ask / quote.mark`

### `OptionPosition`

```text
{
  contract: string,
  snapshot?: OptionSnapshot,
  qty: integer,
  avg_cost: string,
  leg_type: string
}
```

Constraints:

- `qty` uses signed-contract semantics: positive means long and negative means short
- `snapshot` may be omitted in external JSON; Rust core carries it with an empty snapshot default internally
- `avg_cost` keeps price-string semantics and stays aligned with Rust-side `Decimal`
- `leg_type` is a formal canonical model field rather than a migration-only hint
- the lower layer owns derivation of `position_side`, absolute quantity, and canonical contract from `contract + qty + leg_type`

### `ShortItmPosition`

```text
{
  contract: OptionContract,
  quantity: number,
  option_price: number,
  intrinsic: number,
  extrinsic: number
}
```

Notes:

- this is the standard return model for `analysis.short_itm_positions(...)`
- `quantity` is the absolute quantity of the short leg
- missing `option_price` is already normalized to `0`

### `StrategyLegInput`

```text
{
  contract: OptionContract,
  order_side: OrderSide,
  ratio_quantity: number,
  premium_per_contract: number | null
}
```

### `QuotedLeg`

```text
{
  contract: OptionContract,
  order_side: OrderSide,
  ratio_quantity: number,
  quote: OptionQuote,
  snapshot: OptionSnapshot | null
}
```

### `StrategyValuationPosition`

```text
{
  contract: OptionContract,
  quantity: integer,
  avg_entry_price: number | null,
  implied_volatility: number | null
}
```

Notes:

- `quantity` uses signed-contract semantics: positive means long and negative means short
- `avg_entry_price` directly preserves the existing upstream signed cost convention; the library does not derive the sign again
- `implied_volatility` is only used for live revaluation; if a leg is already expired before `evaluation_time`, the lower layer automatically degrades to intrinsic value

### `StrategyPnlInput`

```text
{
  positions: StrategyValuationPosition[],
  underlying_price: number,
  evaluation_time: NyTimestampString,
  entry_cost: number | null,
  rate: number,
  dividend_yield: number | null,
  long_volatility_shift: number | null
}
```

Notes:

- `entry_cost` is the aggregate entry cost for the whole structure; when it is empty, the lower layer sums `avg_entry_price * quantity * 100` for each leg
- `long_volatility_shift` only applies to long, unexpired legs and exists to preserve the current strategy-layer long-IV shock use case

### `StrategyBreakEvenInput`

```text
{
  positions: StrategyValuationPosition[],
  evaluation_time: NyTimestampString,
  entry_cost: number | null,
  rate: number,
  dividend_yield: number | null,
  long_volatility_shift: number | null,
  lower_bound: number,
  upper_bound: number,
  scan_step?: number,
  tolerance?: number,
  max_iterations?: number
}
```

Notes:

- this is the standard input for `payoff.strategy_break_even_points(...)`
- the lower layer owns bracket scanning, root deduplication, and Brent refinement, so higher layers do not maintain parallel scan or bisection implementations

### `ExecutionSnapshot`

```text
{
  contract: string,
  timestamp: NyTimestampString,
  bid: string,
  ask: string,
  price: string,
  greeks: Greeks,
  iv: number
}
```

Notes:

- this is the flat snapshot model used by execution and order-leg flows
- it directly serves the current order payload, price preview, and pre-submit quote display semantics
- it is not the new canonical market-data owner; market-data core still uses `OptionSnapshot`
- `OptionSnapshot -> ExecutionSnapshot` mapping is owned internally by `execution_quote`

### `ExecutionLeg`

```text
{
  symbol: string,
  ratio_qty: string,
  side: OrderSide,
  position_intent: PositionIntent,
  leg_type: string,
  snapshot: ExecutionSnapshot | null
}
```

Notes:

- this is the standard output model for `execution_quote.order_legs` and `execution_quote.roll_legs`
- it directly serves the current order payload and price-preview semantics
- when `leg_type` is not explicitly provided upstream, the lower layer derives it from the sign of `qty` and the contract `option_right`

### `ExecutionLegInput`

```text
{
  action: ExecutionAction,
  leg_type: string,
  contract: string,
  quantity?: number | null,
  snapshot?: ExecutionSnapshot | null,
  timestamp?: NyTimestampString | null,
  bid?: number | null,
  ask?: number | null,
  price?: number | null,
  spread_percent?: number | null,
  greeks?: GreeksInput | null,
  iv?: number | null
}
```

Notes:

- this is the standard input model for `execution_quote.leg(...)`
- it covers page-side pseudo-leg construction when the caller only has `contract + price/spread + greeks`
- if `snapshot` already exists, the lower layer does not require the caller to split out bid, ask, IV, or Greeks again

### `RollLegSelection`

```text
{
  leg_type: string,
  quantity?: number | null
}
```

### `ExecutionQuoteRange`

```text
{
  best_price: number,
  worst_price: number
}
```

Notes:

- positive values mean debit
- negative values mean credit

### `ScaledExecutionQuote`

```text
{
  structure_quantity: number,
  price: number,
  total_price: number,
  total_dollars: number
}
```

### `ScaledExecutionQuoteRange`

```text
{
  structure_quantity: number,
  per_structure: {
    best_price: number,
    worst_price: number
  },
  per_order: {
    best_price: number,
    worst_price: number
  },
  dollars: {
    best_price: number,
    worst_price: number
  }
}
```

Notes:

- this structure directly covers the current front-end requirement to show per-structure price, full-order price, and total dollar amount side by side
- the sign still follows the shared debit and credit convention

### `OptionChain`

```text
{
  underlying_symbol: string,
  as_of: NyTimestampString,
  snapshots: OptionSnapshot[]
}
```

### `OptionChainRecord`

```text
{
  as_of: NyTimestampString,
  underlying_symbol: string,
  occ_symbol: string,
  expiration_date: NyDateString,
  option_right: OptionRight,
  strike: number,
  underlying_price: number | null,
  bid: number | null,
  ask: number | null,
  mark: number | null,
  last: number | null,
  implied_volatility: number | null,
  delta: number | null,
  gamma: number | null,
  vega: number | null,
  theta: number | null,
  rho: number | null
}
```

### `LiquidityOptionData`

```text
{
  contract: string,
  option_type: string,
  strike: number,
  expiration_date: NyDateString,
  dte: integer,
  delta: number,
  spread_pct: number,
  liquidity?: boolean | null,
  bid: number,
  ask: number,
  price: number,
  iv: number
}
```

Notes:

- this is the canonical single-contract model for liquidity heatmaps and chain analysis
- `spread_pct` consistently uses percentage semantics, for example `8.5` means `8.5%`
- `delta` is stored as an absolute value so higher layers do not need to re-handle call versus put sign rules

### `LiquidityStats`

```text
{
  total_count: integer,
  avg_spread_pct: number,
  median_spread_pct: number,
  min_spread_pct: number,
  max_spread_pct: number,
  dte_range: [integer, integer],
  delta_range: [number, number]
}
```

Notes:

- stats logic and empty-input defaults are owned by the lower layer
- median calculation, range bounds, and spread aggregation are not meant to be recomputed separately by page code

### `LiquidityData`

```text
{
  symbol: string,
  timestamp: NyTimestampString,
  underlying_price: number,
  options: LiquidityOptionData[],
  stats: LiquidityStats
}
```

### `LiquidityBatchResponse`

```text
{
  results: Record<string, LiquidityData>
}
```
