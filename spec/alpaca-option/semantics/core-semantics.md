# alpaca-option Core Semantics

## Core Principles

- `alpaca-option` only carries provider-neutral core semantics
- time is always provided by `alpaca-time`, and math functions prefer explicit `years`
- compound inputs prefer object arguments; simple scalar helpers may keep direct signatures so long positional argument lists do not keep spreading

## Contract Semantics

### `parse_occ_symbol`

- must support OCC canonical underlying symbols
- must support OCC-adjusted suffix digits such as `TSLL1`
- does not need to restore display symbols automatically, for example `BRKB -> BRK.B`
- page-level simplified regex parsers are not the long-term owner

### `build_occ_symbol`

- `underlying_symbol` input is always interpreted as OCC canonical form
- if application code passes a display symbol, it must normalize it through `normalize_underlying_symbol` first

### Tolerance lives inside canonical APIs

- `parse_occ_symbol` / `parseOccSymbol` returns `null` or `None` for invalid input
- `build_occ_symbol` / `buildOccSymbol` directly absorbs lenient `option_right` input
- TypeScript `buildOccSymbol` directly accepts `string | number` for `strike`
- the application layer should not keep composing `is_occ_symbol + parse_occ_symbol + try/catch`

## Pricing Semantics

### Pricing model

The current core pricing model is Black-Scholes-Merton:

- `spot`
- `strike`
- `years`
- `rate`
- `dividend_yield`
- `volatility`
- `option_right`

### Time input

Pricing, Greeks, IV, probability, and analysis modules do not directly accept "now". They accept explicit `years`:

- time conversion belongs to `alpaca-time`
- the pricing core does not depend on a system clock

### `implied_volatility_from_price`

- only solves the numeric IV problem
- does not perform time conversion
- `target_price` uses the same per-contract price unit as the theoretical price

## Quote and Execution-Quote Semantics

### `mark`

Recommended `mark` semantics:

- when both `bid` and `ask` exist: `(bid + ask) / 2`
- when only one side exists: degrade to that side
- when both are missing: `mark` may be `null`

Migration notes:

- the canonical owner of the heavily used legacy `snapshot.price` field is `snapshot.quote.mark`
- the core model does not keep a parallel top-level `price` field
- `execution_quote.quote` owns convergence of legacy flat `bid / ask / price` snapshots into `OptionQuote`

### `ExecutionQuoteRange`

The returned net price range follows the Alpaca and current-system convention:

- positive = debit
- negative = credit
- interpolation operates directly on the signed number line rather than on absolute values

### Quantity representation

The new API does not treat signed quantity as the primary model:

- positions: `position_side + quantity`
- legs: `order_side + ratio_quantity`

### `progress_of_limit`

- `progress_of_limit` is the inverse of `limit_quote_by_progress`
- the returned value is clamped into `[0, 1]`
- when `best_price == worst_price`, it returns `0.5`
- this directly serves front-end sliders, numeric inputs, and progress bars

### `order_legs`

- `order_legs` is the sole owner of `position -> execution legs`
- application code should not recompose:
  - `qty` sign handling
  - `side`
  - `position_intent`
  - `ratio_qty`
  - `leg_type` fallback
- `include_leg_types` and `exclude_leg_types` filtering semantics belong in the lower layer
- `close` and `open` are selected by a single `action` parameter rather than by parallel helper names

### `roll_legs`

- `roll_legs` owns both segments of a roll:
  - close the old contract
  - open the new contract
- missing, invalid, or oversized selection quantities are normalized by the lower layer
- when the new snapshot is missing, that leg is skipped directly instead of leaking fallback complexity upward
- `ExecutionSnapshot` only exists as the execution-leg boundary model and does not replace core `OptionSnapshot`

### `leg`

- `leg` is the only builder for a single execution leg
- application code should not manually assemble:
  - `side`
  - `position_intent`
  - `ratio_qty`
  - pseudo-snapshot `bid / ask / price / greeks / iv`
- if only `price + spread_percent` are provided, the lower layer derives bid and ask for the execution snapshot
- this capability specifically serves add, preview, and other page-side pseudo-leg scenarios

## URL Semantics

### OptionStrat URL

Currently supported common format:

- `https://optionstrat.com/build/custom/{underlying_path}/{leg_fragments}`
- `underlying_path` uses `%2F` for symbols such as `BRK.B`

### Leg fragment

Recommended canonical format:

- long: `.SPY250321P560x1@1.10`
- short: `-.SPY250321P580x1@2.45`

Notes:

- direction is encoded by the prefix rather than by the sign of premium
- premium uses absolute value

### Stock fragment

Recommended format:

- `SPYx100@575.12`
- `BRKBx50@498.40`

Notes:

- symbol normalization stays in the lower layer
- the current canonical semantics only accepts positive quantity
- cost is preserved as per-share cost with two decimal places

### URL builders only keep canonical fragments

- `build_optionstrat_leg_fragment` / `buildOptionstratLegFragment`
- `build_optionstrat_stock_fragment` / `buildOptionstratStockFragment`
- `build_optionstrat_url` / `buildOptionstratUrl`
- `merge_optionstrat_urls` / `mergeOptionstratUrls`

The caller must normalize inputs into canonical fragments first:

- option leg:
  - `occ_symbol`
  - `quantity`
  - `premium_per_contract`
- stock fragment:
  - `underlying_symbol`
  - `quantity`
  - `cost_per_share`

Goals:

- keep a single URL-builder input semantics
- converge invalid inputs into `null` or `None`
- keep application calls as one-liners
- if TypeScript receives legacy `positions` directly, the lower layer must normalize them into canonical legs before the builder path
- premium normalization uses explicit cost first; if explicit cost is missing, it falls back to preview `price / mark / last`
- contract normalization uses explicit `occ_symbol` first; when missing, it can build directly from `{ underlying_symbol, expiration_date, strike, option_right }`

`merge_optionstrat_urls` / `mergeOptionstratUrls` is responsible for merging existing URLs back into canonical legs and rebuilding them while absorbing:

- empty URLs
- invalid URLs
- mismatched underlyings
- invalid leg fragments

## Expiration-Selection Semantics

### `nearest_weekly_expiration`

- the meaning is "the nearest valid weekly option expiration"
- it must consider holiday-adjusted weekly expirations instead of mechanically picking the nearest Friday

### `weekly_expirations_between`

- returns a closed interval of weekly expiration dates
- results are sorted ascending by date
- it should cover the current front-end need for six-month candidate expirations
- if a regular Friday is not a valid expiration, the rule falls back to the effective trading-day expiration for that weekly product

## Analysis Semantics

### `annualized_premium_yield`

Use the general form:

- `premium / capital_base / years`

Separate call and put versions are intentionally not part of the API.

### `calendar_forward_factor`

The API directly accepts:

- `short_iv`
- `long_iv`
- `short_years`
- `long_years`

It does not read "today" implicitly.

### `moneyness_label`

The default `atm_band = 0.02`, returning:

- `itm`
- `atm`
- `otm`

### `otm_percent`

This is the most common display-oriented percentage semantics for the front end:

- `call`: `(strike - spot) / spot * 100`
- `put`: `(spot - strike) / spot * 100`

Conventions:

- positive means OTM
- negative means ITM
- it serves UI display and sorting first rather than acting as a strict pricing input

### `assignment_risk`

`assignment_risk` takes remaining extrinsic value and returns:

- `danger`
- `critical`
- `high`
- `medium`
- `low`
- `safe`

This tiering directly supports the current front-end assignment-risk table and badge semantics; thresholds converge inside the library instead of being re-hardcoded by callers.

### `short_extrinsic_amount`

`short_extrinsic_amount` converges the high-frequency semantics of "total remaining extrinsic dollar amount across short legs":

- it only counts short calls and short puts
- long legs are ignored automatically
- when price is missing, the contract is invalid, or no short legs exist, the API directly returns an empty value
- TypeScript directly accepts the legacy `OptionPosition` shape from the main app so page code no longer needs to do its own `parseFloat`, `parseOccSymbol`, or `Math.abs(qty)` guard logic

The goal is not to introduce a new pricing model, but to collapse the repeated short-extrinsic fallback logic into one stable entry point.

### `short_itm_positions`

`short_itm_positions` converges the high-frequency risk and table semantics of "find all short option legs that are ITM":

- long legs are ignored directly
- invalid contracts or invalid quantities are skipped
- missing `option_price` is normalized to `0`
- only legs with `intrinsic > 0` enter the result
- each result directly returns `contract / quantity / option_price / intrinsic / extrinsic`

This keeps active pages and risk pages from manually looping through `parseOccSymbol`, short-leg detection, intrinsic calculation, and `extrinsicValue(...)` calls.

## Adapter Semantics

`alpaca-facade` is responsible for:

- mapping Alpaca provider payloads into core models
- parsing URLs, fetching provider snapshots, and assembling `OptionPosition`

It is not responsible for:

- user-facing client acquisition
- databases, caches, or scheduling
- service orchestration

## Front-end Supplemental Semantics

### `display.format_strike`

- this is a heavily used real helper in the current front end
- integers render without decimal places
- non-integers strip trailing zeros
- precision is kept up to 3 decimals to cover adjusted contracts and similar edge cases

### `display.option_right_code`

- `call -> C`
- `put -> P`

This is a locale-neutral short code rather than UI copy.

### `ScaledExecutionQuoteRange`

Several order dialogs need to show all of the following at once:

- best and worst per-structure prices
- the order price after multiplying by strategy quantity
- the total dollar amount after multiplying by `100`

This means scaled execution quotes are not optional convenience helpers; they are required migration capabilities.

### `numeric`

Even if most business flows eventually consume numeric kernels through `pricing` or `analysis`:

- the main app still has a meaningful `Math::* / Maths.*` migration surface
- front-end realtime calculations need a stable, testable numeric base

For that reason `numeric` remains a public submodule rather than a purely private implementation detail.
