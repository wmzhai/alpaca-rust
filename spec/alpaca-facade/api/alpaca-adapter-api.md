# alpaca-facade Adapter API

This document defines the current adapter boundary exposed by `alpaca-facade`.

## Global Boundary

### Responsibilities

- it may depend on `alpaca-data` and `alpaca-core`
- it maps Alpaca payloads into `alpaca-option` core models
- it repairs missing or obviously invalid provider Greeks and IV values when enough input data exists
- it handles provider symbols versus display symbols
- it enriches OptionStrat URLs into live snapshots

### Non-responsibilities

- user-facing client lifecycle
- databases, caches, or schedulers owned by the application
- service orchestration
- TypeScript-side provider adapters

## Root Exports

- `map_snapshot`
- `map_snapshots`
- `map_snapshot_with_pricing_reference`
- `map_snapshots_with_pricing_references`
- `map_live_snapshots`
- `AlpacaData::get_prices_for_iv_calculation`
- `AlpacaData::map_live_snapshots`
- `pricing_references_for_snapshots`
- `required_underlying_display_symbols`
- `underlying_display_symbols`
- `resolve_positions_from_optionstrat_url`
- `OptionPricingReference`
- `ResolvedOptionStratPositions`
- `SPEC_ADAPTER_API`

## Shared Structure

### `OptionPricingReference`

```text
{
  evaluation_time: String,
  underlying_price: Option<Decimal>
}
```

This structure is the adapter-level pricing context used when provider Greeks
or IV must be repaired. `evaluation_time` is always a New York timestamp in
`YYYY-MM-DD HH:MM:SS` form.

### `ResolvedOptionStratPositions`

```text
{
  underlying_display_symbol: String,
  legs: Vec<StrategyLegInput>,
  positions: Vec<OptionPosition>
}
```

## `map_snapshot`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_snapshot(occ_symbol, snapshot, underlying_price?, dividend_yield?)` | `OptionSnapshot` | maps a single Alpaca `Snapshot` into a core `OptionSnapshot` |

Current behavior:

- invalid `occ_symbol` returns `invalid_occ_symbol`
- if `snapshot.timestamp()` is missing, `as_of` falls back to the current New York timestamp
- provider timestamps are normalized into `YYYY-MM-DD HH:MM:SS`
- bid, ask, mark, and last converge into `OptionQuote`
- if provider Greeks or IV are missing or obviously invalid, the adapter repairs them when a valid pricing reference is available
- regular-session repair evaluates at the option snapshot timestamp
- non-regular-session repair evaluates at the last completed trading date at `16:00:00`
- `map_snapshot(...)` and `map_snapshots(...)` do not fetch close prices; they wrap caller-provided `underlying_price` values in the appropriate pricing-reference time
- valid provider IV is preserved and is not re-inferred from price
- ultra-low-price options apply conservative Greeks clipping to avoid propagating unstable values

## `map_snapshot_with_pricing_reference`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_snapshot_with_pricing_reference(occ_symbol, snapshot, pricing_reference?, dividend_yield?)` | `OptionSnapshot` | maps a single Alpaca `Snapshot` using an explicit pricing reference |

Current behavior:

- it is the explicit-context form of `map_snapshot(...)`
- callers can use it when they already resolved the exact evaluation time and spot
- the returned `OptionSnapshot.underlying_price` is the pricing reference spot

## `map_snapshots`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_snapshots(snapshots, underlying_prices?, dividend_yield?)` | `OptionSnapshot[]` | batch-maps a provider snapshot map |

Current behavior:

- output order stays stable via `alpaca_data::options::ordered_snapshots(...)`
- `underlying_prices` may be keyed by display symbol, for example `BRK.B`
- they may also be keyed by OCC or provider-canonical underlying, for example `BRKB`
- each contract automatically selects the most appropriate underlying price

## `map_snapshots_with_pricing_references`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_snapshots_with_pricing_references(snapshots, pricing_references?, dividend_yield?)` | `OptionSnapshot[]` | batch-maps provider snapshots using explicit per-contract pricing references |

Current behavior:

- output order stays stable via `alpaca_data::options::ordered_snapshots(...)`
- each contract uses the pricing reference keyed by its OCC symbol
- missing pricing references leave missing/invalid provider Greeks or IV unrepaired

## `AlpacaData::get_prices_for_iv_calculation`

| API | Returns | Semantics |
| --- | --- | --- |
| `AlpacaData::get_prices_for_iv_calculation(symbols)` | `HashMap<String, Decimal>` | resolves each stock symbol to the stock price used by IV and Greeks calculation |

Current behavior:

- symbols are normalized to display form
- during regular session, it performs one cache-backed batch stock snapshot request via `CachedClient::stocks(...)`
- during regular session, it returns the positive realtime snapshot `Decimal` price per symbol
- outside regular session, it performs one batch `bars_all(...)` request for the last completed trading day's daily bars
- outside regular session, it returns the positive daily-bar `Decimal` close for that completed trading date
- it does not loop over symbols for provider requests
- it does not expose an `f64` stock-price map

## `map_live_snapshots`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_live_snapshots(snapshots, underlying_prices?, dividend_yield?)` | `OptionSnapshot[]` | pure batch mapping from provider snapshots and already loaded pricing references |
| `AlpacaData::map_live_snapshots(snapshots, known_prices?, dividend_yield?)` | `OptionSnapshot[]` | batch-maps provider snapshots and resolves missing stock-price references through `get_prices_for_iv_calculation(...)` |

Current behavior:

- the adapter decides the pricing-reference mode from the current New York regular session state
- `AlpacaData::map_live_snapshots(...)` first fetches required symbols through `AlpacaData::get_prices_for_iv_calculation(...)`
- caller-provided `Decimal` prices are only used as a supplement for symbols that the unified price entry did not return
- outside regular session, fallback IV and repaired Greeks use the last completed trading day's daily-bar close as the reference spot
- provider price-fetch failures from `get_prices_for_iv_calculation(...)` propagate instead of falling back to a different stock-price source
- mapping reuses `map_snapshots_with_pricing_references(...)`, preserving ordering, symbol lookup, and repair rules

## `pricing_references_for_snapshots`

| API | Returns | Semantics |
| --- | --- | --- |
| `pricing_references_for_snapshots(snapshots, realtime_prices?, close_prices?, now)` | `HashMap<String, OptionPricingReference>` | resolves per-contract pricing references from preloaded price maps |

Current behavior:

- regular session uses option snapshot timestamps and realtime prices
- non-regular session uses the last completed trading date at `16:00:00` and close prices
- prices may be keyed by display symbol such as `BRK.B` or provider-canonical underlying such as `BRKB`

## `required_underlying_display_symbols`

| API | Returns | Semantics |
| --- | --- | --- |
| `required_underlying_display_symbols(snapshots)` | `String[]` | returns the display symbols that still need underlying prices for Greeks or IV repair |

Current behavior:

- results are deduplicated and sorted
- output uses display symbols rather than OCC-canonical symbols
- share-class symbols such as `BRK.B` remain in display form

## `resolve_positions_from_optionstrat_url`

| API | Returns | Semantics |
| --- | --- | --- |
| `resolve_positions_from_optionstrat_url(value, client)` | `ResolvedOptionStratPositions` | parses an OptionStrat URL into live-enriched strategy legs and positions |
| `AlpacaData::resolve_optionstrat_url(value)` | `(String, Vec<OptionPosition>)` | parses an OptionStrat URL and enriches positions through the cache-backed facade |

Current behavior:

- the core layer only owns URL parsing and leg-fragment parsing
- the adapter layer uses `alpaca_data::Client` to fetch provider snapshots directly
- `AlpacaData::resolve_optionstrat_url(...)` fetches raw provider snapshots, resolves stock prices through `get_prices_for_iv_calculation(...)`, and applies URL premiums as the IV-calculation source when present
- returned position snapshots include `underlying_price` when the facade can resolve a valid stock-price reference
- enrichment stays on the unified snapshot-repair path instead of duplicating provider fallback in higher layers
- provider request failures are normalized to `provider_snapshot_fetch_failed`
- stock-price request failures propagate from `get_prices_for_iv_calculation(...)`
- missing provider snapshots return `missing_provider_snapshot`

## Division of Responsibilities with the Core Layer

- contract, pricing, URL, execution quote, and snapshot semantics are defined by `alpaca-option`
- the adapter only absorbs provider payloads and performs live enrichment
- new provider fallback with reuse value should converge inside the adapter rather than being pushed up into application code
