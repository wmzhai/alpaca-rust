# Project Structure

`alpaca-rust` is organized around published Rust crates, optional workspace
companions, and supporting documentation/test assets.

## Published Rust Crates

| Path | Published crate | Role | Notes |
| --- | --- | --- | --- |
| `crates/alpaca-core` | `alpaca-core` | Shared primitives | Credentials, base URLs, query helpers, serde helpers |
| `crates/alpaca-http` | `alpaca-rest-http` | Shared transport | Directory path differs from the published crate name |
| `crates/alpaca-data` | `alpaca-data` | Market Data HTTP SDK | Stocks, options, news, corporate actions |
| `crates/alpaca-trade` | `alpaca-trade` | Trading HTTP SDK | Account, orders, positions, activities, contracts, watchlists |
| `crates/alpaca-mock` | `alpaca-mock` | Executable mock server | Market-data-backed trade validation |
| `crates/alpaca-time` | `alpaca-time` | Time semantics | New York time, trading calendar, expiration helpers |
| `crates/alpaca-option` | `alpaca-option` | Option semantics | Contracts, snapshots, pricing, payoff, URL helpers |
| `crates/alpaca-facade` | `alpaca-facade` | Convenience facade | High-level composition of the lower workspace crates |

## Optional Workspace Plus Features

| Path | Package | Role |
| --- | --- | --- |
| `packages/alpaca-time` | `@alpaca/time` | Optional TypeScript companion for time semantics |
| `packages/alpaca-option` | `@alpaca/option` | Optional TypeScript companion for option semantics |

These TypeScript packages are available to workspace consumers, but they are not
the primary published system surface or the default public entry point.

## Supporting Directories

| Path | Role |
| --- | --- |
| `docs/` | Public user-facing documentation |
| `spec/` | Crate-scoped API, model, and semantics contracts |
| `memory/` | Local collaboration memory and routing notes |
| `fixtures/` | Shared JSON fixtures for `alpaca-time` and `alpaca-option` |
| `tests/support/live/` | Shared live-test support for Rust crates |
| `tools/api-coverage/` | API coverage manifests and audit tooling |
| `tools/docs/` | Documentation metadata generation scripts |
| `website/` | Docusaurus site for public docs and rustdoc hosting |

## Layering

### Foundation SDK

- `alpaca-core`
- `alpaca-rest-http`
- `alpaca-data`
- `alpaca-trade`
- `alpaca-mock`

### Semantic Core

- `alpaca-time`
- `alpaca-option`

### Convenience Facade

- `alpaca-facade`

## Important Naming Note

The source directory `crates/alpaca-http` publishes as the crate
`alpaca-rest-http`. Public docs, docs.rs links, and release automation use the
published crate name; repository layout uses the source directory name.
