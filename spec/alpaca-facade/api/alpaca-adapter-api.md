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
- `map_live_snapshots`
- `required_underlying_display_symbols`
- `resolve_positions_from_optionstrat_url`
- `ResolvedOptionStratPositions`
- `SPEC_ADAPTER_API`

## Shared Structure

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
| `map_snapshot(occ_symbol, snapshot, underlying_price?, risk_free_rate?, dividend_yield?)` | `OptionSnapshot` | maps a single Alpaca `Snapshot` into a core `OptionSnapshot` |

Current behavior:

- invalid `occ_symbol` returns `invalid_occ_symbol`
- if `snapshot.timestamp()` is missing, `as_of` falls back to the current New York timestamp
- provider timestamps are normalized into `YYYY-MM-DD HH:MM:SS`
- bid, ask, mark, and last converge into `OptionQuote`
- if provider Greeks or IV are missing or obviously invalid, the adapter repairs them when a valid `underlying_price` is available
- ultra-low-price options apply conservative Greeks clipping to avoid propagating unstable values

## `map_snapshots`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_snapshots(snapshots, underlying_prices?, risk_free_rate?, dividend_yield?)` | `OptionSnapshot[]` | batch-maps a provider snapshot map |

Current behavior:

- output order stays stable via `alpaca_data::options::ordered_snapshots(...)`
- `underlying_prices` may be keyed by display symbol, for example `BRK.B`
- they may also be keyed by OCC or provider-canonical underlying, for example `BRKB`
- each contract automatically selects the most appropriate underlying price

## `map_live_snapshots`

| API | Returns | Semantics |
| --- | --- | --- |
| `map_live_snapshots(snapshots, client, underlying_prices?, risk_free_rate?, dividend_yield?)` | `OptionSnapshot[]` | batch-maps provider snapshots and pulls missing underlying stock snapshots when needed |

Current behavior:

- callers may pass already known `underlying_prices`; the adapter only fetches missing data
- the fetch scope covers every underlying display symbol present in the input snapshots
- stock snapshot failures degrade to "use only the prices we already have" so callers do not need to re-implement fallback logic
- mapping still reuses `map_snapshots(...)`, preserving ordering, symbol lookup, and repair rules

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

Current behavior:

- the core layer only owns URL parsing and leg-fragment parsing
- the adapter layer uses `alpaca_data::Client` to fetch provider snapshots directly
- it first fetches any underlying stock snapshots required for repair, then reuses `map_live_snapshots(...)`
- returned position snapshots try to include `underlying_price` whenever possible
- enrichment stays on the unified snapshot-repair path instead of duplicating provider fallback in higher layers
- provider request failures are normalized to `provider_snapshot_fetch_failed`
- underlying stock request failures degrade to mapped results without newly fetched prices
- missing provider snapshots return `missing_provider_snapshot`

## Division of Responsibilities with the Core Layer

- contract, pricing, URL, execution quote, and snapshot semantics are defined by `alpaca-option`
- the adapter only absorbs provider payloads and performs live enrichment
- new provider fallback with reuse value should converge inside the adapter rather than being pushed up into application code
