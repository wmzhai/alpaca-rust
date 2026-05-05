# alpaca-option Public API

This document defines the current canonical public API for `alpaca-option`.

Notes:

- the document is grouped by semantic API area
- Rust and TypeScript may differ slightly in host-level input shapes; those differences are absorbed in the semantic description of each entry
- if a host-specific signature differs from this document but preserves the same semantics, this document remains the canonical shared contract

## Global Constraints

- Rust uses `snake_case`
- TypeScript uses `camelCase`
- root exports must stay mirrored across Rust and TypeScript
- each semantic capability keeps a single public name
- tolerance, fallback, and lenient parsing must converge inside the canonical API itself
- provider-specific logic does not enter the core public API
- Rust core may depend on `alpaca-core` for float and decimal helpers, but those low-level details do not become provider-facing API surface

## Root Exports

### Shared core modules

- `analysis`
- `chain`
- `contract`
- `display`
- `executionQuote` / `execution_quote`
- `expirationSelection` / `expiration_selection`
- `numeric`
- `payoff`
- `pricing`
- `probability`
- `snapshot`
- `url`

### Shared types and errors

- `OptionError`
- Rust: `OptionResult`
- all core model types

### Shared namespace with a separate contract

- `math`
  - also exported from the root today
  - its stable mirrored contract is defined in `internal-math-kernels.md`

## Shared Types

### Scalar and enum semantics

- `OptionRight`: `call | put`
- `OptionRightCode`: `C | P`
- `OrderSide`: `buy | sell`
- `PositionSide`: `long | short`
- `ExecutionAction`: `open | close`
- `PositionIntent`: `buy_to_open | sell_to_open | buy_to_close | sell_to_close`
- `MoneynessLabel`: `itm | atm | otm`
- `AssignmentRiskLevel`: `danger | critical | high | medium | low | safe`

### Key structural types

- `OptionContract`
- `OptionQuote`
- `ContractDisplay`
- `Greeks`
- `BlackScholesInput`
- `BlackScholesImpliedVolatilityInput`
- `OptionSnapshot`
- `OptionPosition`
- `ShortItmPosition`
- `StrategyLegInput`
- `QuotedLeg`
- `GreeksInput`
- `ExecutionSnapshot`
- `ExecutionLeg`
- `RollLegSelection`
- `RollRequest`
- `ExecutionLegInput`
- `ExecutionQuoteRange`
- `ScaledExecutionQuote`
- `ScaledExecutionQuoteRange`
- `ParsedOptionStratUrl`
- `OptionStratLegInput`
- `OptionStratStockInput`
- `OptionStratUrlInput`
- `OptionChain`
- `OptionChainRecord`
- `LiquidityOptionData`
- `LiquidityStats`
- `LiquidityData`
- `LiquidityBatchResponse`
- `PayoffLegInput`
- `StrategyValuationPosition`
- `StrategyPnlInput`
- `StrategyBreakEvenInput`

Field-level definitions live in `../models/core-models.md`.

## Error Conventions

- Rust validation-oriented APIs usually return `OptionResult<T>` or `Option<T>`
- TypeScript validation-oriented APIs usually return `null`, or throw `OptionError` for strict numeric functions
- public capabilities with fallback behavior absorb that fallback internally; parallel helpers such as `OrNull`, `Lossy`, or `Safe` are intentionally not part of the contract

## `contract`

### Responsibilities

- underlying-symbol normalization
- OCC-symbol validation, parsing, and construction
- canonical contract convergence

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `contract.normalize_underlying_symbol(symbol)` / `contract.normalizeUnderlyingSymbol(symbol)` | `string` | normalizes a display symbol into OCC form, for example `BRK.B -> BRKB` |
| `contract.is_occ_symbol(occ_symbol)` / `contract.isOccSymbol(occSymbol)` | `boolean` | whether the string is a parseable OCC symbol |
| `contract.parse_occ_symbol(occ_symbol)` / `contract.parseOccSymbol(occSymbol)` | `OptionContract \| null` | invalid OCC input returns an empty value; adjusted OCC suffixes such as `TSLL1` are supported |
| `contract.build_occ_symbol(underlying_symbol, expiration_date, strike, option_right)` / `contract.buildOccSymbol(underlyingSymbol, expirationDate, strike, optionRight)` | `string \| null` | automatically handles underlying normalization and `call/put/c/p`; invalid strike or right returns an empty value |
| `contract.canonical_contract(input)` / `contract.canonicalContract(input)` | `OptionContract \| null` | converges an OCC symbol, canonical contract, or legacy contract-like input into a standard contract |

## `pricing`

### Responsibilities

- Black-Scholes-Merton price, Greeks, and IV
- intrinsic and extrinsic value
- contract-driven extrinsic value

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `pricing.price_black_scholes(input)` / `pricing.priceBlackScholes(input)` | `number` | returns the theoretical price from `BlackScholesInput` |
| `pricing.greeks_black_scholes(input)` / `pricing.greeksBlackScholes(input)` | `Greeks` | returns `delta/gamma/vega/theta/rho` from `BlackScholesInput` |
| `pricing.implied_volatility_from_price(input)` / `pricing.impliedVolatilityFromPrice(input)` | `number` | solves IV from a target option price |
| `pricing.intrinsic_value(spot, strike, option_right)` / `pricing.intrinsicValue(spot, strike, optionRight)` | `number` | returns intrinsic value |
| `pricing.extrinsic_value(option_price, spot, strike, option_right)` / `pricing.extrinsicValue(optionPrice, spot, strike, optionRight)` | `number` | returns `max(option_price - intrinsic_value, 0)` |
| `pricing.contract_extrinsic_value(option_price, spot, contract)` / `pricing.contractExtrinsicValue(optionPrice, spot, contract)` | `number \| null` | directly accepts canonical contract or legacy contract input and returns an empty value for invalid cases |

## `probability`

### Responsibilities

- probability that the underlying finishes inside a range at expiration

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `probability.expiry_probability_in_range(input)` / `probability.expiryProbabilityInRange(input)` | `number` | computes the probability from spot, bounds, years, rate, dividend yield, and volatility |

## `analysis`

### Responsibilities

- annualized yield
- moneyness
- OTM percentage
- assignment risk
- remaining extrinsic and ITM classification for short legs
- strike for a target delta

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `analysis.annualized_premium_yield(...)` / `analysis.annualizedPremiumYield(...)` | `number` | annualized yield based on premium, capital base, and years |
| `analysis.annualized_premium_yield_days(...)` / `analysis.annualizedPremiumYieldDays(...)` | `number` | annualized yield based on calendar days |
| `analysis.calendar_forward_factor(...)` / `analysis.calendarForwardFactor(...)` | `number` | calendar-forward factor from near and far IV terms |
| `analysis.moneyness_ratio(spot, strike)` / `analysis.moneynessRatio(spot, strike)` | `number` | `spot / strike` |
| `analysis.moneyness_label(spot, strike, option_right, atm_band?)` / `analysis.moneynessLabel(spot, strike, optionRight, atmBand?)` | `MoneynessLabel` | returns `itm/atm/otm` |
| `analysis.otm_percent(spot, strike, option_right)` / `analysis.otmPercent(spot, strike, optionRight)` | `number` | positive means OTM and negative means ITM |
| `analysis.position_otm_percent(spot, position)` / `analysis.positionOtmPercent(spot, position)` | `number \| null` | directly accepts position-shaped or legacy contract-shaped input |
| `analysis.assignment_risk(extrinsic)` / `analysis.assignmentRisk(extrinsic)` | `AssignmentRiskLevel` | returns a risk tier based on remaining extrinsic value |
| `analysis.short_extrinsic_amount(spot, positions, structure_quantity?)` / `analysis.shortExtrinsicAmount(spot, positions, structureQuantity?)` | `number \| null` | totals the remaining extrinsic dollar amount across short legs |
| `analysis.short_itm_positions(spot, positions)` / `analysis.shortItmPositions(spot, positions)` | `ShortItmPosition[]` | returns all short option legs that are ITM at the current spot |
| `analysis.strike_for_target_delta(...)` / `analysis.strikeForTargetDelta(...)` | `number` | solves for the strike that matches a target delta |

## `payoff`

### Responsibilities

- single-leg expiration payoff
- multi-leg expiration payoff
- expiration break-even points

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `payoff.single_leg_payoff_at_expiry(input)` / `payoff.singleLegPayoffAtExpiry(input)` | `number` | single-leg expiration PnL |
| `payoff.strategy_payoff_at_expiry({ legs, underlying_price_at_expiry })` / `payoff.strategyPayoffAtExpiry({ legs, underlyingPriceAtExpiry })` | `number` | multi-leg expiration PnL |
| `payoff.break_even_points({ legs })` / `payoff.breakEvenPoints({ legs })` | `number[]` | sorted expiration break-even points |

## `option_strategy`

### Responsibilities

- strategy mark-to-market PnL
- strategy-level break-even search
- strategy-level curve sampling
- strategy-level Greeks aggregation

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `option_strategy.strategy_pnl(input)` / `optionStrategy.strategyPnl(input)` | `number` | revalues the full position set at `evaluation_time`; expired legs use intrinsic value, live legs use BSM; when `entry_cost` is omitted, the implementation sums `avg_entry_price * quantity * 100` for each leg |
| `option_strategy.strategy_break_even_points(input)` / `optionStrategy.strategyBreakEvenPoints(input)` | `number[]` | searches for strategy-level PnL roots inside `[lower_bound, upper_bound]` using scan plus Brent refinement and returns sorted break-even points |
| `OptionStrategy` | class / struct | prepares reusable strategy valuation state for mark value, PnL, curve sampling, break-even scanning, and Greeks aggregation |

## `chain`

### Responsibilities

- filtering snapshots by contract criteria
- finding a single snapshot
- producing expiration-date lists

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `chain.list_snapshots(...)` / `chain.listSnapshots(...)` | `OptionSnapshot[]` | supports filtering by OCC symbol or by `expiration + strike + right`; preserves the original chain order |
| `chain.find_snapshot(...)` / `chain.findSnapshot(...)` | `OptionSnapshot \| null` | returns the first matching snapshot |
| `chain.expiration_dates(...)` / `chain.expirationDates(...)` | `{ expiration_date, calendar_days }[]` | auto-deduplicates and sorts by `calendar_days` and `expiration_date` |

## `execution_quote`

### Responsibilities

- unified quote extraction
- `ExecutionLeg` construction
- order-leg and roll-leg construction
- roll-request normalization
- best/worst quote ranges and limit prices derived from progress

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `execution_quote.quote(input)` / `executionQuote.quote(input)` | `OptionQuote \| null` | accepts `quote`, `snapshot`, `position`, `leg`, and legacy flat quote payloads |
| `execution_quote.limit_price(input)` / `executionQuote.limitPrice(input)` | `number` | accepts `execution.limit_price`, `price`, string values, and empty values |
| `execution_quote.leg_type(input)` / `executionQuote.legType(input)` | `string \| null` | normalizes `side + position_intent + contract` into `longcall/shortcall/longput/shortput` |
| `execution_quote.roll_request(input)` / `executionQuote.rollRequest(input)` | `RollRequest \| null` | accepts either `target_contract` or `new_strike + new_expiration` |
| `execution_quote.leg(input)` / `executionQuote.leg(input)` | `ExecutionLeg \| null` | canonical builder for a single execution leg |
| `execution_quote.order_legs(input)` / `executionQuote.orderLegs(input)` | `ExecutionLeg[]` | builds open or close legs from positions |
| `execution_quote.roll_legs(input)` / `executionQuote.rollLegs(input)` | `ExecutionLeg[]` | builds roll legs from old positions and a target snapshot |
| `execution_quote.best_worst(input, structure_quantity?)` / `executionQuote.bestWorst(input, structureQuantity?)` | `ScaledExecutionQuoteRange` | accepts positions or legs and returns the best/worst range |
| `execution_quote.scale_quote(input)` / `executionQuote.scaleQuote(input)` | `ScaledExecutionQuote` | scales a single price by structure quantity |
| `execution_quote.scale_quote_range(input)` / `executionQuote.scaleQuoteRange(input)` | `ScaledExecutionQuoteRange` | scales a range by structure quantity |
| `execution_quote.limit_quote_by_progress(input)` / `executionQuote.limitQuoteByProgress(input)` | `number` | `progress = 0` means best price and `1` means worst price; percentage-style inputs are also accepted |
| `execution_quote.progress_of_limit(input)` / `executionQuote.progressOfLimit(input)` | `number` | maps a limit quote back into `[0, 1]` progress |

## `snapshot`

### Responsibilities

- extracting contracts from snapshots
- spread and spread percent
- basic snapshot validity
- liquidity classification

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `snapshot.contract(snapshot)` | `OptionContract \| null` | accepts both canonical and legacy `snapshot.contract` shapes |
| `snapshot.spread(snapshot)` | `number` | missing bid or ask is absorbed as `0` |
| `snapshot.spread_pct(snapshot)` / `snapshot.spreadPct(snapshot)` | `number` | returns `0` when the price is close to `0` |
| `snapshot.is_valid(snapshot)` / `snapshot.isValid(snapshot)` | `boolean` | returns `true` when the contract is valid and `asOf/timestamp` is non-empty |
| `snapshot.liquidity(snapshot)` | `boolean \| null` | combines spread, delta, and DTE into a liquidity classification |

## `url`

### Responsibilities

- OptionStrat underlying paths
- OptionStrat leg fragments, stock fragments, and full URLs
- URL merging
- URL and leg-fragment parsing

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `url.to_optionstrat_underlying_path(symbol)` / `url.toOptionstratUnderlyingPath(symbol)` | `string` | converts a display symbol into an OptionStrat path |
| `url.from_optionstrat_underlying_path(path)` / `url.fromOptionstratUnderlyingPath(path)` | `string` | converts an OptionStrat path back into a display symbol |
| `url.build_optionstrat_leg_fragment(input)` / `url.buildOptionstratLegFragment(input)` | `string \| null` | builds a single option-leg fragment |
| `url.build_optionstrat_stock_fragment(input)` / `url.buildOptionstratStockFragment(input)` | `string \| null` | builds a stock fragment |
| `url.build_optionstrat_url(input)` / `url.buildOptionstratUrl(input)` | `string \| null` | builds a full OptionStrat URL |
| `url.merge_optionstrat_urls(...)` / `url.mergeOptionstratUrls(...)` | `string \| null` | merges multiple URLs |
| `url.parse_optionstrat_url(url)` / `url.parseOptionstratUrl(url)` | `ParsedOptionStratUrl` | parses the base URL structure |
| `url.parse_optionstrat_leg_fragments(...)` / `url.parseOptionstratLegFragments(...)` | `StrategyLegInput[]` | parses leg fragments into canonical strategy legs |

## `display`

### Responsibilities

- strike formatting
- compact contract display
- option-right codes

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `display.format_strike(strike)` / `display.formatStrike(strike)` | `string` | removes insignificant trailing zeros |
| `display.position_strike(position)` / `display.positionStrike(position)` | `string` | returns `-` when the position cannot be parsed |
| `display.compact_contract(contract?, expiration_format?)` / `display.compactContract(contract?, expirationFormat?)` | `string` | returns values such as `615C@04-19`; returns `-` when parsing fails |
| `display.contract_display(contract?, expiration_format?)` / `display.contractDisplay(contract?, expirationFormat?)` | `ContractDisplay \| null` | returns `{ strike, expiration, compact, optionRightCode }` |
| `display.option_right_code(option_right)` / `display.optionRightCode(optionRight)` | `OptionRightCode` | `call -> C`, `put -> P` |

## `expiration_selection`

### Responsibilities

- weekly expiration selection
- standard monthly expirations

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `expiration_selection.nearest_weekly_expiration(anchor_date)` / `expirationSelection.nearestWeeklyExpiration(anchorDate)` | `string` | returns the nearest weekly expiration in the week containing the anchor date |
| `expiration_selection.weekly_expirations_between(start_date, end_date)` / `expirationSelection.weeklyExpirationsBetween(startDate, endDate)` | `string[]` | returns weekly expirations inside the range |
| `expiration_selection.standard_monthly_expiration(year, month)` / `expirationSelection.standardMonthlyExpiration(year, month)` | `string` | returns the standard third Friday expiration |

## `numeric`

### Responsibilities

- normal-distribution helpers
- rounding
- `linspace`
- Brent solving

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `numeric.normal_cdf(x)` / `numeric.normalCdf(x)` | `number` | standard normal CDF |
| `numeric.normal_pdf(x)` / `numeric.normalPdf(x)` | `number` | standard normal PDF |
| `numeric.round(value, decimals)` | `number` | rounds to a fixed number of decimal places |
| `numeric.linspace(start, end, count)` | `number[]` | generates an evenly spaced sequence |
| `numeric.brent_solve(...)` / `numeric.brentSolve(...)` | `number` | Brent root finding |

## `math`

The `math` namespace is currently exposed from the root and includes:

- `math.black76`
- `math.bachelier`
- `math.american`
- `math.barrier`
- `math.geometricAsian` / `math.geometric_asian`

Its stable mirrored contract is documented separately in `internal-math-kernels.md`.

## Adapter Boundary

Alpaca-specific mapping capabilities are intentionally not part of the core public API. Their contract lives in:

- `../../alpaca-facade/api/alpaca-adapter-api.md`
