# Workspace Companion Packages

`alpaca-rust` keeps a small set of optional TypeScript packages inside the
workspace. They are used for frontend or tooling reuse and are not the primary
release API surface.

## Which Rust crates have TypeScript companions

| Rust crate | TS package | Package name | What it provides | Coverage notes |
| --- | --- | --- | --- | --- |
| `alpaca-time` | `packages/alpaca-time` | `@alpaca/time` | Time/calendar/expiration/session/display helpers with namespace exports (`clock`, `calendar`, `session`, `expiration`, `range`, `display`, `browser`) | Optional frontend parity for time-domain calculations |
| `alpaca-option` | `packages/alpaca-option` | `@alpaca/option` | Option-domain semantic helpers for contracts, snapshots, pricing, payoff, probability, execution quotes, and strategy helpers | Rich in tests + bounded public API exports |
| `alpaca-trade` | `packages/alpaca-trade` | `@alpaca/trade` | Shared execution-type model (`Execution`) used by frontend order models | Does **not** provide a full HTTP client |

## Which Rust crates have no TypeScript companion

- `alpaca-core`
- `alpaca-rest-http`
- `alpaca-data`
- `alpaca-mock`
- `alpaca-facade`

These are Rust-only published crates in the current workspace release model.

## Package boundaries

- All workspace packages are `private: true` and are not published as independent
  npm artifacts as part of the current release process.
- They are validated locally through workspace scripts in `package.json`.
- `packages/alpaca-option` also has dedicated TS tests for API boundaries and
  fixture metadata format to keep behavior stable.

## Workspace validation commands

- `pnpm run test:packages` — runs TS tests for `@alpaca/time` and
  `@alpaca/option`
- `pnpm --filter @alpaca/time test`
- `pnpm --filter @alpaca/option test`
- `pnpm --filter @alpaca/trade typecheck`
- `pnpm run typecheck:packages` — type checks `@alpaca/time` and
  `@alpaca/option`
- `pnpm --filter @alpaca/time typecheck`
- `pnpm --filter @alpaca/option typecheck`

## Relation to docs and release

- `docs` and `release-checklist` describe Rust crates as the official published
  surface.
- TypeScript companions are documented as optional support packages for frontend
  reuse and consistency with backend models.
