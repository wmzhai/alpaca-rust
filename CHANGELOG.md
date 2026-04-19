# Changelog

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
- Added release guards that require tags to point at `master` and match the workspace version before publish steps can run.

## v0.23.8

- Prepared the initial public `alpaca-rust` workspace release line.
- Added publish-ready package metadata for `alpaca-core`, `alpaca-rest-http`, `alpaca-data`, `alpaca-trade`, and `alpaca-mock`.
- Added public README, license files, and crate-level documentation baselines.
- Expanded public documentation to describe implemented API coverage, unsupported scopes, and crate boundaries.
- Added resource-level reference guides plus installation, testing, mock-server, and troubleshooting documentation.
