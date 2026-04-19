# New York Time and U.S. Trading Calendar Semantics

## Core Principles

- everything is interpreted in `America/New_York`
- public strings only use:
  - dates: `YYYY-MM-DD`
  - timestamps: `YYYY-MM-DD HH:MM:SS`
- business callers must not invent a second layer of time helpers or fallback semantics

## Dual Date and Timestamp Semantics

### Both input shapes are valid business representations

This crate accepts both:

- `NyDateString`
- `NyTimestampString`

They represent different levels of precision:

- a date means "a specific day"
- a timestamp means "a specific moment in New York time"

Date-only values are therefore not malformed timestamps and must not be treated that way.

### Parsing preserves the original granularity

- `clock.parse_date(...)` only accepts dates
- `clock.parse_timestamp(...)` accepts timestamps or RFC3339 UTC and outputs a canonical New York timestamp
- `clock.parse_date_or_timestamp(...)` preserves the original granularity
- `clock.first_date_or_timestamp(...)` returns the first valid candidate and preserves its granularity

### Same-day ordering must not invent intraday precedence

`clock.compare_date_or_timestamp(...)` follows these rules:

- compare natural dates first
- if both values are full timestamps on the same day, compare down to the time-of-day
- if either side is date-only on the same day, treat them as the same ordering tier

This avoids forcing date-only data into invented times such as `00:00:00` or `16:00:00`.

## Market Session Semantics

On trading days:

- `premarket`: `04:00:00 <= t < 09:30:00`
- `regular`: `09:30:00 <= t < regular_close`
- `after_hours`: `regular_close <= t < 20:00:00`
- `closed`: all other times

Where:

- `regular_close = 16:00:00` on normal trading days
- `regular_close = 13:00:00` on early-close days
- non-trading days are `closed` for the full day

An additional provider-facing helper remains available:

- `session.is_overnight_window(timestamp)`
- defined as `20:00:00 <= t` or `t < 04:00:00`
- this does not extend `MarketSession`; it only serves overnight-feed routing and checks

## Holidays and Early Close Days

The implementation directly covers:

- regular NYSE market holidays
- early-close days such as the day after Thanksgiving, the day before Independence Day, and Christmas Eve
- observed holidays when a holiday falls on a weekend

Rust and TypeScript must remain behaviorally aligned. If they diverge, fix the implementation rather than changing the contract.

### Trading-day offsets

- `calendar.add_trading_days(date, days)` is the only public trading-date offset API
- `days > 0` moves forward, `days < 0` moves backward, and `days = 0` returns the canonical original date
- separate direction-specific public helpers such as `previous_*` or `next_*` are intentionally not part of the contract

## Expiration Semantics

### `expiration.close(expiration_date)`

Option expiration is always defined as the regular close of that trading date:

- normal day: `YYYY-MM-DD 16:00:00`
- early-close day: `YYYY-MM-DD 13:00:00`

### `expiration.calendar_days(expiration_date, at)`

This is the display-oriented semantics:

- across different dates, use natural calendar-day difference
- return `0` on the expiration date before close
- return `-1` after the close has passed

### `expiration.days(expiration_date, at)`

This is the precise time-based semantics:

- compute `(close(expiration_date) - at) / 86400`
- negative values are allowed
- use canonical New York wall-clock strings for the difference

### `expiration.years(expiration_date, at, basis)`

- derived from `days(...)`
- default basis is `ACT/365.25`
- always returns a non-negative value
- invalid inputs or already expired values return `0`

### `expiration.years_between_dates(start_date, end_date, basis)`

- based only on pure date differences
- default basis is `ACT/365`
- does not inject expiration-close semantics

## Display Semantics

### `display.compact(input, style)`

One interface covers both date-only and timestamp display:

- date styles render compact dates
- timestamp styles behave as follows:
  - timestamp input -> compact timestamp output
  - date-only input -> automatically falls back to the matching date format
- invalid inputs are returned unchanged

### `display.time(input, precision, date_style)`

- timestamp input -> `HH:MM` or `HH:MM:SS`
- date-only input -> compact date
- invalid inputs are returned unchanged

### `display.duration(start, end)`

- only accepts `HH:MM` semantics
- invalid inputs return `-`

### `display.weekdayShortZh(date)`

- this is a TypeScript-only UI helper
- it is not part of the mirrored core contract

## Date Range Semantics

- `range.add_days(...)` only applies calendar-day offsets
- `range.dates(...)` and `range.trading_dates(...)` return closed intervals
- `range.nth_weekday(...)` selects the nth occurrence of a weekday
- `range.is_last_trading_date_of_week(...)` means the date itself is a trading day and its next trading day belongs to the next natural week
- `range.weekly_last_trading_dates(...)` filters by the rule above
- `range.calendar_week_range(...)` and `range.iso_week_range(...)` both return Monday-through-Sunday spans according to their own calendar system

The crate deliberately does not expose a "current-time-to-range" facade. When callers need ranges anchored on today, they should explicitly combine `clock.today()` with `range.add_days(...)`.

For "today +/- N days", compose the APIs directly:

- Rust: `range::add_days(&clock::today(), n)`
- TypeScript: `range.addDays(clock.today(), n)`

## TypeScript Companion Semantics

The companion layer only keeps front-end-specific capabilities:

- `display.weekdayShortZh(...)`
- `browser.dateObjectToNyDate(...)`

Any behavior that used to require separate fallback wrappers now belongs inside the canonical APIs themselves.

## Testing Conventions

- static behavior should be regression-tested through `fixtures/`
- wrappers such as `now()`, `today()`, and `is_regular_session_now()` rely on runtime tests
- RFC3339 UTC, DST, early close days, and mixed date-only versus timestamp ordering remain permanent high-risk edge cases
