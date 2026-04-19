# Fixtures

`fixtures/` stores cross-language shared samples that lock down critical Rust and
TypeScript time semantics.

## Layout

- `parsing/`: dates, timestamps, RFC3339, and mixed-granularity parsing
- `calendar/`: trading dates, holidays, and early closes
- `session/`: premarket / regular / after-hours / overnight
- `expiration/`: expiration cutoff times, DTE, and year fractions
- `range/`: calendar date lists, trading date lists, and week ranges
- `display/`: compact date-time and duration rendering
- `dst/`: DST boundary conversions

## Conventions

- Each fixture file covers one topic.
- The `api` field names the canonical API under test.
- Inputs and outputs use public string contracts instead of language-specific types.
- Use `tolerance` when a numeric result allows a small error margin.
- Runtime wrappers such as `now_*` are validated with controlled clock tests instead of brittle static samples.

## Usage

- `spec/` defines what the semantics should be.
- `fixtures/` shows what the critical boundaries look like.
- Rust and TypeScript tests regress these samples together so DST, early-close, and date-only mixed-input behavior does not drift again.
