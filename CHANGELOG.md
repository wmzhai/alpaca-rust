# Changelog

## Unreleased

- Added a Rust-only `alpaca-option` Black-Scholes Gamma API that preserves the existing pricing validation and rate contract while avoiding price, normal CDF, and full-Greeks calculations.
- Switched all `alpaca-mock` parent and nested order IDs to raw UUID v4 values and added cross-account create/replace regression coverage, preventing concurrent virtual accounts from generating colliding broker order IDs.
- Resolved failed-terminal replacements to their filled predecessors during both immediate replace completion and later effective-order synchronization, preventing rejected replacements from hiding real fills.
- Added a standalone `alpaca-mock` replacement-race fixture and network regression coverage for the broker ordering race.

## v0.30.0

- Aligned the adopted non-crypto Trading REST surface with Alpaca Trading API `2.0.1`, binding all 38 adopted operations to their exact source, mock route, request, response, status, and network-scenario contracts.
- Added the distinct activity-by-type endpoint, typed asset/calendar/clock/portfolio contracts, order cursors and nested reads, advanced order instructions, corrected position-close responses, complete watchlist lifecycles, and option exercise/do-not-exercise support.
- Added `SubmitOrderRequest` builders for client order IDs and explicit simple-order position intent.
- Made client-ID creates recoverable after ambiguous request failures with recovered-order shape validation.
- Made strict recreate transitions require a stable client order ID and preserve parent or nested child fill evidence instead of creating a duplicate replacement.
- Aligned fractional-share order validation with observed Paper behavior: create requests accept `day` market, limit, stop, and stop-limit orders, while replacement quantities remain whole-share only.
- Replaced recursive order legs with typed `OrderLeg` values, made order reads accept an explicit `GetRequest`, reused `Order` for position-close responses, and aligned public account, asset, position, and watchlist fields with current requiredness and typed values.
- Expanded `alpaca-mock` into a stateful HTTP implementation for all 38 adopted operations, including assets, option contracts, calendars, clocks, portfolio history, configurations, watchlists, strict order status codes, and position instructions.
- Replaced local, fixture-based, silent-skip, and in-process API-crate tests with fail-fast network-only suites that exercise canonical APIs or a standalone mock service through the public clients. Thirty-seven operations completed full Paper/mock scenario closure; do-not-exercise has a verified raw Paper strict-empty `200` and a passing mock scenario, while its Paper exact-scenario cleanup replay remains explicitly unclaimed.
- Updated the crate README, Trading coverage baseline, testing guide, mainline guide, and resource references for the new contracts and migration points.

## v0.29.0

- Aligned the adopted surface with Alpaca Market Data API `1.1`, including conditional dispatch to all eight single-symbol stock routes while preserving the canonical public request and response APIs.
- Expanded Corporate Actions with typed regions, all 15 action types, currency, ISIN, subtype, partial-call, reorganization, and nested stock-movement fields plus complete pagination merging.
- Strengthened Market Data coverage auditing and real Paper-key integration tests, while deferring Index endpoints until the required entitlement is available.
- Raised the workspace Rust version and pinned toolchain and GitHub Pages builds to Rust `1.97.0`.

## v0.28.0

- Added `dealer_view` market-structure exposure mode alongside the existing `gex_proxy` mode.
- Added mode-aware market-structure analysis options in the Rust crate and TypeScript package while preserving `gex_proxy` as the default.
- Changed market-structure call and put wall selection to rank by absolute gamma strength so dealer-view sign reversal does not fall back to open interest.
- Removed deprecated PDT/DTBP fields from the `alpaca-trade` account and account-configuration models after Alpaca removed them from Trading API responses.
- Added `crypto_status` to the `alpaca-trade` account model and aligned `alpaca-mock` account responses plus API coverage metadata with the current Trading API schema.

## v0.27.2

- Fixed `alpaca-mock` to reject non-positive simple order and replace `limit_price` values while preserving negative `mleg` credit limit prices.
- Added regression coverage for simple create/replace rejection and `mleg` credit limit acceptance.

## v0.27.1

- Preserved signed OptionStrat URL premiums in the Rust and TypeScript helpers so calibrated residual legs can emit costs such as `@-1.92`.
- Added public API boundary coverage for building, parsing, and merging signed OptionStrat premiums.

## v0.27.0

- Bumped the publishable Rust workspace, website metadata, generated docs metadata, and public install snippets to `0.27.0`.
- Updated the public `alpaca-trade` reference to document the current activity, order lifecycle, and position reconciliation helpers.
- Updated the public `alpaca-option` reference and API spec to cover market structure, liquidity model types, risk-free-rate exports, option-strategy state, and numeric search helpers.
- Aligned testing, examples, troubleshooting, and release docs with the current mainline test names, mock lifecycle example, pnpm website build command, and publishing order.

## v0.26.1

- Added a static Treasury risk-free-rate curve and wired Black-Scholes defaults, option strategy mark/Greeks, and probability calculations to use term-specific rates.
- Added matching TypeScript rate exports for `@alpaca/option`.
- Reworked OptionStrat URL imports to use URL premiums as IV calculation inputs and repair per-leg IV/Greeks with explicit pricing references.
- Preserved caller-supplied option valuation spot prices while mapping live snapshots, only fetching missing symbols through the IV calculation price source.
- Added SMH diagonal fixed-IV payoff regression coverage.

- Fixed non-regular option valuation stock prices to fetch completed trading-day closes from SIP regular-session bars, avoiding BOATS overnight daily timestamps being filtered out as the wrong date.
- Added `AlpacaData::get_prices_for_iv_calculation` as the single Decimal stock-price source for IV and Greeks calculation, using realtime stock snapshots during regular sessions and one batched daily-bars request outside regular sessions.
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
