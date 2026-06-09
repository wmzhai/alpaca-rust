# Changelog

## Unreleased

- Fixed non-regular option valuation stock prices to fetch completed trading-day closes from SIP regular-session bars, avoiding BOATS overnight daily timestamps being filtered out as the wrong date.
- Added `AlpacaData::get_prices_for_option` as the single Decimal stock-price source for option valuation, using realtime stock snapshots during regular sessions and one batched daily-bars request outside regular sessions.
- Reworked facade option snapshot mapping so underlying spot references flow through Decimal maps and only convert to `f64` at the `alpaca-option` pricing boundary.
- Changed option market data mirror Greeks and implied volatility to deserialize as finite `f64` values instead of `Decimal`, keeping fixed-precision decimals for prices and cash-like fields.
- Aligned `alpaca-facade` option snapshot fallback pricing so repaired IV and Greeks use explicit pricing references: regular-session repair uses realtime stock snapshots, while non-regular repair uses the latest stock daily-bar close at the last completed trading-day close timestamp.
- Documented the new pricing-reference helpers and daily-bars-only non-regular close behavior.
- Raised the repository Rust toolchain, workspace `rust-version`, and GitHub Actions Rust setup to `1.96.0`.
- Normalized `Execution` order prices to two decimal places for limit, dynamic-limit, and dynamic-market flows before submission or progress reporting.
- Updated OptionStrat URL helpers to accept signed short quantities such as `.IWM260605C285x-2@0.9` while preserving legacy `-.CONTRACTx2` parsing.
- Changed generated short option-leg fragments to the signed-quantity form `.CONTRACTx-2@...` in the Rust and TypeScript helpers.

## v0.26.0

- Bumped the entire publishable workspace to `0.26.0` and aligned all in-workspace dependency pins.
- Updated all public versioned snippets and release metadata (README/docs/release checklist/website metadata).
- Regenerated doc metadata output to keep docs pages consistent with the new version.

## v0.25.4

- Added realtime option strategy peak fields for exposing current-curve maximum profit price, profit, and unit mark value.
- Reworked `OptionStrategy` realtime PnL peak search to scan the full configured price interval and refine the best candidate, avoiding local-peak misses on diagonal/calendar curves.
- Mirrored the peak search and serialization changes in the TypeScript package and documented the expanded public API.

## v0.25.3

- Added shared `OptionPosition` helpers for snapshot-based construction, model input overrides, quantity scaling, and effective IV fallback.
- Extended `OptionStrategy` with serializable strategy-owned state fields plus reusable break-even search, PnL peak search, and break-even de-duplication primitives for downstream strategy implementations.
- Mirrored the new option strategy helpers in the TypeScript package and documented the expanded `alpaca-option` public API.

## v0.25.2

- Simplified option strategy valuation inputs so public payoff, break-even, curve, and model Greeks APIs use `OptionPosition` directly.
- Added a root `VERSION` file initialized from the current workspace version.
- Removed the `alpaca-facade` option-chain convenience API so option-chain callers use the lower-level `alpaca-data` client directly.
- Added default risk-free rate semantics for option pricing helpers and related probability/payoff flows.
- Bumped the full Rust workspace, website package metadata, generated docs metadata, release checklist, and public install snippets to `0.25.2`.

## v0.25.1

- Bumped the full Rust workspace, website package metadata, and public install snippets to `0.25.1`.
- Fixed stale documentation site examples that still showed older crate versions on Getting Started and Installation pages.
- Kept GitHub Pages deployment in the tag-only release flow while aligning release guards to `main`.
- Added exact option expiration request support so option-data callers can request a specific contract expiration instead of only range-style filtering.
- Exported shared stock symbol helpers from `alpaca-trade` for downstream strategy and application code.
- Improved live tooling dotenv discovery by walking parent directories for the workspace environment file.
- Reduced generated TypeScript binding noise for option positions and tightened OptionStrat parsing around implicit quantities and non-custom build URLs.
- Carried forward the release workflow guard fix so tag releases validate against `main`, matching the current repository default branch.

## v0.24.8

- Removed publish-time version pins from cross-crate `dev-dependencies` so release packaging no longer forms a registry cycle between `alpaca-rest-http`, `alpaca-data`, `alpaca-trade`, and `alpaca-mock`.
- Simplified the release pipeline to focus on docs generation, site builds, and crate publication, leaving full Rust compilation and test coverage outside the release path.
- Rolled the partially published `v0.24.7` attempt forward into the next patch release so all public crates can ship from one clean source version.

## v0.24.7

- Aligned the repository toolchain override with the published workspace floor by changing `rust-toolchain.toml` from `1.95.0` to `1.94.1` and installing `rustfmt` there as well.
- Rolled the failed `v0.24.6` tag attempt forward into the next patch release so the release workflow can execute from a corrected tagged commit.

## v0.24.6

- Lowered the library workspace `rust-version` floor from `1.95.0` to `1.94.1` after confirming the published Rust crates still build and pass their key tests on `1.94.1`.
- Fixed the release workflow Rust toolchain setup so `cargo fmt --check` runs with the required `rustfmt` component installed.
- Rolled the failed `v0.24.5` tag attempt forward into the next patch release so the release workflow can rerun from a corrected tagged commit.

## v0.24.5

- Reorganized the public documentation and website around the current three-layer Rust workspace: foundation SDK crates, semantic crates, and the `alpaca-facade` composition layer.
- Added publish-ready crate metadata and public READMEs for `alpaca-time`, `alpaca-option`, and `alpaca-facade`.
- Clarified that the TypeScript workspace packages are optional plus features for repo consumers, not the primary published system surface.
- Extended the release checklist, docs metadata generation, and GitHub release automation to cover all eight published Rust crates.
- Removed the remaining `optworks`-specific Rust export hooks from the published crates and aligned the workspace toward the `0.24.5` release line.

## v0.24.4

- Added a workspace installer script for running `alpaca-mock` as a local user service on macOS and Ubuntu.
- Documented the service installer in the root README, installation guide, mock server guide, and crate README.

## v0.24.3

- Fixed the tag release workflow so GitHub Release notes are extracted without the shell heredoc parsing failure seen in prior tags.
- Added a concrete patch-release runbook that captures version bumps, preflight checks, verification commands, and recovery steps for future `alpaca-rust` releases.

## v0.24.2

- Fixed the release workflow YAML so GitHub Actions can load the tag-only pipeline and publish crates from a valid workflow definition.

## v0.24.1

- Fixed the tag-release publish workflow so the crates.io publish step can resolve crate versions without a shell heredoc parsing failure.

## v0.24.0

- Migrated the public documentation site to the Docusaurus workspace layout with unified `alpaca-rust` branding and multi-crate docs.rs navigation.
- Consolidated release automation into one tag-only GitHub Actions workflow that validates the workspace, publishes crates in dependency order, deploys GitHub Pages, and creates the GitHub Release.
- Added release guards that require tags to point at `main` and match the workspace version before publish steps can run.

## v0.23.8

- Prepared the initial public `alpaca-rust` workspace release line.
- Added publish-ready package metadata for `alpaca-core`, `alpaca-rest-http`, `alpaca-data`, `alpaca-trade`, and `alpaca-mock`.
- Added public README, license files, and crate-level documentation baselines.
- Expanded public documentation to describe implemented API coverage, unsupported scopes, and crate boundaries.
- Added resource-level reference guides plus installation, testing, mock-server, and troubleshooting documentation.
