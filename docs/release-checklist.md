# Release Checklist

This checklist defines the public release bar for `alpaca-rust`.

## Public Surface

- Root `README.md` reflects the current workspace shape
- Each published crate has its own `README.md`
- `Cargo.toml` metadata is present for every published crate
- docs.rs is the primary API-reference host
- GitHub Pages hosts narrative docs for the whole workspace
- `CHANGELOG.md` contains a non-empty section for the release tag
- author metadata uses `Weiming Zhai <wmzhai@gmail.com>`

## Verification

Run before a release:

```bash
cargo fmt --check
cargo check --workspace
cargo test --doc
cargo doc --workspace --no-deps
npm run build --prefix website
cargo package --list -p alpaca-core
cargo package --list -p alpaca-http
cargo package --list -p alpaca-data
cargo package --list -p alpaca-trade
cargo package --list -p alpaca-mock
cargo publish --dry-run -p alpaca-core
cargo publish --dry-run -p alpaca-http
cargo publish --dry-run -p alpaca-data
cargo publish --dry-run -p alpaca-trade
cargo publish --dry-run -p alpaca-mock
```

Before the first real release, downstream `cargo publish --dry-run` commands are expected to require previously published workspace dependencies on crates.io.

## Live Verification

Run the release-confidence live suite appropriate for the current market state:

- `cargo test -p alpaca-data --tests -- --nocapture`
- `cargo test -p alpaca-trade --test mainline_api -- --nocapture`
- `cargo test -p alpaca-trade --test orders_mock_contract -- --nocapture`
- `cargo test -p alpaca-trade --test positions_mock_contract -- --nocapture`
- `cargo test -p alpaca-mock -- --nocapture`

## Publishing Order

Publish in dependency order:

1. `alpaca-core`
2. `alpaca-http`
3. `alpaca-data`
4. `alpaca-trade`
5. `alpaca-mock`

Wait for crates.io index visibility before publishing each dependent crate.

## GitHub Automation

- The first crates.io publication may be manual
- Subsequent tag releases should use crates.io Trusted Publishing
- GitHub Pages should deploy from GitHub Actions
