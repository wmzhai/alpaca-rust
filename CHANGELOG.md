# Changelog

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
