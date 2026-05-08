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
  leg_type: string,
  option_right?: "call" | "put",
  strike?: number,
  valuation_years?: number
}
```

Constraints:

- `qty` uses signed-contract semantics: positive means long and negative means short
- `snapshot` may be omitted in external JSON; Rust core carries it with an empty snapshot default internally
- `avg_cost` keeps price-string semantics and stays aligned with Rust-side `Decimal`
- `leg_type` is a formal canonical model field rather than a migration-only hint
- `option_right`, `strike`, and `valuation_years` are optional prepared valuation fields filled by `OptionStrategy::prepare`
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

### `StrategyPnlInput`

```text
{
  positions: OptionPosition[],
  qty: integer,
  underlying_price: number,
  evaluation_time: NyTimestampString,
  entry_cost: number | null,
  dividend_yield: number | null
}
```

Notes:

- `positions` use the same `OptionPosition` model as live strategy holdings; the OCC symbol is parsed from `contract`, and runtime valuation inputs such as IV come from the position `snapshot`
- `qty` is the strategy-level quantity and is distinct from each position's signed ratio `qty`
- `entry_cost` is the aggregate entry cost for the whole structure; when it is empty, the lower layer sums `avg_cost * position.qty * 100` for each leg and multiplies by strategy `qty`
- strategy valuation uses `DEFAULT_RISK_FREE_RATE` internally and uses `position.snapshot.implied_volatility` directly

### `StrategyBreakEvenSideInput`

```text
{
  pivot: number,
  boundary: number,
  scan_step: number,
  tolerance?: number,
  max_iterations?: number
}
```

Notes:

- this is a low-level helper input for a single-side BE scan
- strategy layers choose their own pivot and boundary; the core helper only performs bracket scanning and root refinement
- it intentionally does not model complete strategy-specific BE semantics such as open-side flags

### `StrategyPnlPeakSearchInput`

```text
{
  current_price: number,
  step_hint?: number | null,
  left_boundary: number,
  right_boundary: number,
  tolerance?: number | null,
  max_search_steps?: number | null
}
```

Notes:

- this is a low-level helper input for current-curve PnL peak search
- the helper follows the side where PnL improves and returns a nearby positive peak
- callers still decide how that peak maps to finite BE, open-side flags, and display fields

### `StrategyPnlPeak`

```text
{
  spot: number,
  pnl: number
}
```

Notes:

- this result is strategy-agnostic and only reports the spot/PnL pair
- it does not contain finite BE fields or open-side semantics

### `StrategyPositionTotals`

```text
{
  value: number,
  cost: number,
  spread: number,
  spread_rate?: number
}
```

Notes:

- this result aggregates flat `OptionPosition` legs with the 100-share contract multiplier
- `value` uses current snapshot mark price, `cost` uses `avg_cost`, and `spread` sums bid-ask spread by absolute leg quantity
- strategy layers still own how these totals map onto strategy-specific display fields

### `OptionStrategy`

```text
{
  positions: OptionPosition[],
  qty: integer,
  underlying_price: number,
  greeks: Greeks,
  cost: number,
  value: number,
  pnl: number,
  cashflow?: number,
  spread?: number,
  spread_rate?: number,
  max_profit?: number,
  max_loss?: number,
  buying_power?: number,
  pnl_at_expire?: number,
  short_expire_delta?: number,
  break_even_points: number[],
  realtime_break_even_points: number[],
  break_even_low_open: boolean,
  break_even_high_open: boolean,
  break_even_low_distance_percent: number,
  break_even_high_distance_percent: number,
  break_even_width?: number,
  break_even_width_percent: number,
  realtime_break_even_low_open: boolean,
  realtime_break_even_high_open: boolean,
  realtime_break_even_low_distance_percent: number,
  realtime_break_even_high_distance_percent: number,
  realtime_break_even_width?: number,
  realtime_break_even_width_percent: number,
  realtime_max_profit_price?: number,
  realtime_max_profit?: number,
  realtime_max_profit_unit_value?: number,
  short_expiration?: string,
  long_expiration?: string,
  short_dte?: integer,
  long_dte?: integer,
  win_rate?: number,
  theta_rate?: number,
  theta_total?: number,
  score?: number,
  rank?: integer,
  url?: string
}
```

Constraints:

- this is the serializable, TypeScript-exported state container for strategy-level option metrics
- `positions` is the flat position list used by valuation; downstream business-specific views should be derived from it
- `qty` is strategy-level quantity, while each `OptionPosition.qty` is a signed leg ratio
- `underlying_price` is the only current-underlying field; do not add `current_underlying_price`
- build/preview flows may call `calculate_cost_from_positions()` explicitly, while runtime flows should preserve true cost/cashflow from storage or orders
- generic break-even helpers may write the shared break-even fields, but downstream strategies still own boundary selection and open-side interpretation
- realtime peak fields are optional downstream-owned display metrics; `realtime_max_profit_unit_value` is the peak mark value divided by contract multiplier and strategy quantity

### `StrategyBreakEvenInput`

```text
{
  positions: OptionPosition[],
  qty: integer,
  evaluation_time: NyTimestampString,
  entry_cost: number | null,
  dividend_yield: number | null,
  lower_bound: number,
  upper_bound: number,
  scan_step?: number,
  tolerance?: number,
  max_iterations?: number
}
```

Notes:

- this is the standard input for `option_strategy.strategy_break_even_points(...)`
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
