# Changelog

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
