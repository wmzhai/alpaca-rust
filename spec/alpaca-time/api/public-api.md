# alpaca-time Public API

This document defines the current canonical public API for `alpaca-time`.

Notes:

- the document is grouped by semantic API area
- small Rust / TypeScript host differences are called out explicitly inside each section
- if a host-specific signature differs from this document but preserves the same semantics, this document remains the canonical shared contract

## Global Constraints

- Rust uses `snake_case`
- TypeScript uses `camelCase`
- root exports must stay mirrored across Rust and TypeScript
- each semantic capability keeps a single public name
- tolerance, fallback, and lenient parsing must converge inside the canonical API itself
- platform-specific companion helpers are limited to the explicitly listed cases

## Root Exports

### Shared modules

- `clock`
- `calendar`
- `session`
- `expiration`
- `range`
- `display`

### Platform companions

- TypeScript: `browser`
- Rust: `chrono.date(...)`
- Rust: `chrono.timestamp(...)`
- TypeScript: `display.weekdayShortZh(...)`
- TypeScript: `browser.dateObjectToNyDate(...)`

### Shared exports

- Rust: `TimeError`, `TimeResult`
- TypeScript: `TimeError`
- Rust and TypeScript: shared type definitions

## Shared Types

### Scalar types

- `NyDateString`: `YYYY-MM-DD`
- `NyTimestampString`: `YYYY-MM-DD HH:MM:SS`
- `ComparableTimeInput`: `NyDateString | NyTimestampString`
- `HhmmString`: `HH:MM`
- `WeekdayCode`: `mon | tue | wed | thu | fri | sat | sun`
- `MarketSession`: `premarket | regular | after_hours | closed`
- `DayCountBasis`: `ACT/365 | ACT/365.25 | ACT/360`
- `DurationSign`: `-1 | 0 | 1`

### Structural types

#### `MarketHours`

```text
{
  date: NyDateString,
  is_trading_date: boolean,
  is_early_close: boolean,
  premarket_open: HhmmString | null,
  regular_open: HhmmString | null,
  regular_close: HhmmString | null,
  after_hours_close: HhmmString | null
}
```

#### `TradingDayInfo`

```text
{
  date: NyDateString,
  is_trading_date: boolean,
  is_market_holiday: boolean,
  is_early_close: boolean,
  market_hours: MarketHours
}
```

#### `DurationParts`

```text
{
  sign: -1 | 0 | 1,
  total_seconds: number,
  days: number,
  hours: number,
  minutes: number,
  seconds: number
}
```

#### `DateRange`

```text
{
  start_date: NyDateString,
  end_date: NyDateString
}
```

#### `TimestampParts`

```text
{
  date: NyDateString,
  timestamp: NyTimestampString,
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second: number,
  hhmm: number,
  hhmm_string: HhmmString,
  weekday_from_sunday: number
}
```

## Error Conventions

- Rust validation-oriented APIs usually return `TimeResult<T>`
- TypeScript validation-oriented APIs usually throw `TimeError` for invalid inputs
- a small set of APIs includes built-in fallback behavior; those rules are documented in the relevant module sections

## `clock`

### Responsibilities

- current New York time
- canonical date and timestamp normalization
- RFC3339 UTC and Unix-seconds conversion
- mixed-granularity comparison
- minute-level keys and `HH:MM` helpers

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `clock.now()` | `NyTimestampString` | returns the current New York timestamp |
| `clock.today()` | `NyDateString` | returns the current New York date |
| `clock.parse_date(input)` / `clock.parseDate(input)` | canonical date | validates and normalizes a date |
| `clock.parse_timestamp(input)` / `clock.parseTimestamp(input)` | canonical timestamp | accepts canonical timestamps and RFC3339 UTC values |
| `clock.parts(input?)` | `TimestampParts` | reads the current time when omitted |
| `clock.parse_date_or_timestamp(input)` / `clock.parseDateOrTimestamp(input)` | `ComparableTimeInput` | parses as date first, then as timestamp on failure |
| `clock.first_date_or_timestamp(inputs)` / `clock.firstDateOrTimestamp(inputs)` | `ComparableTimeInput \| null` | returns the first parseable non-empty input |
| `clock.to_utc_rfc3339(input)` / `clock.toUtcRfc3339(input)` | `string` | converts a New York timestamp to UTC RFC3339 |
| `clock.from_unix_seconds(seconds)` / `clock.fromUnixSeconds(seconds)` | canonical timestamp or empty string | returns an empty string for `0`, non-integers, or invalid values |
| `clock.truncate_to_minute(input)` / `clock.truncateToMinute(input)` | `string` | zeroes seconds for valid timestamps; best-effort trims partially valid inputs to minute precision; returns the original input if trimming is impossible |
| `clock.minute_key(input)` / `clock.minuteKey(input)` | `YYYY-MM-DD HH:MM` | returns a canonical minute-level key |
| `clock.hhmm_string_from_parts(hour, minute)` / `clock.hhmmStringFromParts(hour, minute)` | `HhmmString` | builds `HH:MM` from parts |
| `clock.minutes_from_hhmm(input)` / `clock.minutesFromHhmm(input)` | `number` | converts `HH:MM` into total minutes |
| `clock.compare_date_or_timestamp(left, right)` / `clock.compareDateOrTimestamp(left, right)` | Rust: `Ordering`, TS: `-1 \| 0 \| 1` | compares dates and timestamps together; falls back to lexical ordering when empty strings appear |
| `clock.fractional_days_between(start, end)` / `clock.fractionalDaysBetween(start, end)` | Rust: `f64`, TS: `number \| null` | treats date-only values as `00:00:00` for the calculation |
| `clock.fractional_days_until(target)` / `clock.fractionalDaysUntil(target)` | Rust: `f64`, TS: `number \| null` | returns the day difference from now to the target |
| `clock.fractional_days_since(input)` / `clock.fractionalDaysSince(input)` | Rust: `f64`, TS: `number \| null` | returns the day difference from the input to now |

### Rust companion

| API | Returns | Semantics |
| --- | --- | --- |
| `chrono.date(input?)` | `NaiveDate` | accepts dates, canonical timestamps, and RFC3339 UTC |
| `chrono.timestamp(input?)` | `NaiveDateTime` | accepts canonical timestamps and RFC3339 UTC |

## `calendar`

### Responsibilities

- trading-day detection
- holiday and early-close metadata
- market-session schedules for a specific date
- last completed trading day
- trading-day offsets

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `calendar.trading_day_info(date)` / `calendar.tradingDayInfo(date)` | `TradingDayInfo` | returns the full trading-day record for a given date |
| `calendar.is_trading_date(date)` / `calendar.isTradingDate(date)` | `boolean` | returns `false` for invalid dates |
| `calendar.is_trading_today()` / `calendar.isTradingToday()` | `boolean` | based on the current New York date |
| `calendar.market_hours_for_date(date)` / `calendar.marketHoursForDate(date)` | `MarketHours` | returns the premarket, regular, and after-hours schedule for that date |
| `calendar.last_completed_trading_date(at_timestamp?)` / `calendar.lastCompletedTradingDate(atTimestamp?)` | `NyDateString` | uses the current New York time when omitted |
| `calendar.add_trading_days(date, days)` / `calendar.addTradingDays(date, days)` | `NyDateString` | supports both positive and negative offsets |

## `session`

### Responsibilities

- market-session classification
- premarket, regular, and after-hours detection
- arbitrary `HH:MM` window checks
- overnight checks

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `session.market_session_at(timestamp)` / `session.marketSessionAt(timestamp)` | `MarketSession` | returns `closed` on non-trading days |
| `session.is_premarket_at(timestamp)` / `session.isPremarketAt(timestamp)` | `boolean` | returns `false` for invalid inputs |
| `session.is_regular_session_at(timestamp)` / `session.isRegularSessionAt(timestamp)` | `boolean` | returns `false` for invalid inputs |
| `session.is_after_hours_at(timestamp)` / `session.isAfterHoursAt(timestamp)` | `boolean` | returns `false` for invalid inputs |
| `session.is_in_window(timestamp, start, end)` / `session.isInWindow(timestamp, start, end)` | `boolean` | uses a closed-open interval `[start, end)` and returns `false` for invalid inputs |
| `session.is_overnight_window(timestamp)` / `session.isOvernightWindow(timestamp)` | `boolean` | currently defined as `20:00-03:59` |
| `session.is_regular_session_now()` / `session.isRegularSessionNow()` | `boolean` | whether the current time is inside the regular session |
| `session.is_overnight_now()` / `session.isOvernightNow()` | `boolean` | whether the current time is inside the overnight window |
| `session.is_in_window_now(start, end)` / `session.isInWindowNow(start, end)` | `boolean` | whether the current time falls inside the given window |

## `expiration`

### Responsibilities

- expiration closing timestamp
- calendar days and fractional days
- year fractions
- annualized time between dates

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `expiration.close(expiration_date)` / `expiration.close(expirationDate)` | `NyTimestampString` | returns the New York close time for the expiration date |
| `expiration.calendar_days(expiration_date, at?)` / `expiration.calendarDays(expirationDate, at?)` | integer | returns `0` on the expiration date before close and `-1` after close |
| `expiration.days(expiration_date, at?)` / `expiration.days(expirationDate, at?)` | floating-point number | returns fractional days and may be negative |
| `expiration.years(expiration_date, at?, basis?)` / `expiration.years(expirationDate, at?, basis?)` | `number` | non-negative; both Rust and TypeScript converge to `0` on invalid input |
| `expiration.years_between_dates(start_date, end_date, basis?)` / `expiration.yearsBetweenDates(startDate, endDate, basis?)` | `number` | computes annualized time using dates only |

## `range`

### Responsibilities

- calendar-date ranges
- trading-date ranges
- week ranges and last trading dates of a week
- nth-weekday calculations

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `range.add_days(date, days)` / `range.addDays(date, days)` | `NyDateString` | calendar-day offset |
| `range.dates(start_date, end_date)` / `range.dates(startDate, endDate)` | `NyDateString[]` | closed-interval date list |
| `range.trading_dates(start_date, end_date)` / `range.tradingDates(startDate, endDate)` | `NyDateString[]` | keeps only trading days |
| `range.nth_weekday(year, month, weekday, nth)` / `range.nthWeekday(year, month, weekday, nth)` | `NyDateString` | returns the `nth` weekday |
| `range.is_last_trading_date_of_week(date)` / `range.isLastTradingDateOfWeek(date)` | `boolean` | returns `false` for invalid dates |
| `range.weekly_last_trading_dates(start_date, end_date)` / `range.weeklyLastTradingDates(startDate, endDate)` | `NyDateString[]` | last trading date of each week in the range |
| `range.last_trading_date_of_week(date)` / `range.lastTradingDateOfWeek(date)` | `NyDateString` | last trading date of the week containing the given date |
| `range.calendar_week_range(date)` / `range.calendarWeekRange(date)` | `DateRange` | Sunday-to-Saturday week range |
| `range.iso_week_range(year, week)` / `range.isoWeekRange(year, week)` | `DateRange` | ISO week range |

## `display`

### Responsibilities

- compact date and timestamp display
- `HH:MM` formatting
- duration text and structured durations
- weekday codes and Chinese short weekdays

### API

| API | Returns | Semantics |
| --- | --- | --- |
| `display.compact(input, style)` | `string` | supports `mm-dd`, `yy-mm-dd`, `yymmdd`, `mm-dd hh:mm`, `yy-mm-dd hh:mm`, and `yyyy-mm-dd hh:mm`; falls back to the original value on invalid input |
| `display.time(input, precision?, date_style?)` / `display.time(input, precision?, dateStyle?)` | `string` | returns a compact date for date-only inputs and the time component for timestamp inputs; falls back to the original value on invalid input |
| `display.hhmm(input)` | `HhmmString` | accepts both `HH:MM` and `HHMM` |
| `display.duration(start, end)` | `string` | returns values such as `2h 30m` or `45m`; returns `-` for invalid input |
| `display.weekday_code(date)` / `display.weekdayCode(date)` | `WeekdayCode` | returns the canonical weekday code |
| `display.duration_parts(seconds)` / `display.durationParts(seconds)` | `DurationParts` | decomposes a second count |
| `display.relative_duration_parts(from, to)` / `display.relativeDurationParts(from, to)` | `DurationParts` | structured duration between two timestamps |
| `display.compact_duration(parts, style?)` / `display.compactDuration(parts, style?)` | `string` | supports `hm` and `dhm` |
| `display.compact_days_until(days)` / `display.compactDaysUntil(days)` | `string` | shows hours for values below one day, otherwise uses `D` |

### TypeScript companion

| API | Returns | Semantics |
| --- | --- | --- |
| `display.weekdayShortZh(date)` | one of seven single-character Chinese weekday labels | TypeScript UI helper for compact Chinese weekday display |

## `browser`

### TypeScript companion

| API | Returns | Semantics |
| --- | --- | --- |
| `browser.dateObjectToNyDate(date)` | `NyDateString` | converts a browser `Date` object into a New York date string |
