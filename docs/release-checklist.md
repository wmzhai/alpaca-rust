# Release Checklist

This checklist defines the public release bar for `alpaca-rust`.

## Current Release Baseline

- The current published release line is `v0.24.4`
- The next minimal patch release should be `v0.24.8`
- `.github/workflows/github-pages.yml` is the only release workflow
- The release workflow triggers only on semantic version tags matching `v*.*.*`
- crates.io Trusted Publishing must remain configured for all published Rust crates:
  `alpaca-core`, `alpaca-rest-http`, `alpaca-data`, `alpaca-trade`,
  `alpaca-mock`, `alpaca-time`, `alpaca-option`, and `alpaca-facade`
- The `github-pages` environment must allow deployments from `master` and tags matching `v*.*.*`
- First-time crates may require one manual `cargo publish` bootstrap before Trusted Publishing can be enabled on crates.io

## Public Surface

- The official published system surface is the Rust workspace only
- `packages/alpaca-time` and `packages/alpaca-option` remain optional workspace plus features and are not separate release artifacts
- Root `README.md` reflects the current three-layer Rust workspace shape
- Each published Rust crate has its own `README.md`
- `Cargo.toml` metadata is present for every published Rust crate
- docs.rs is the primary API-reference host
- GitHub Pages hosts narrative docs for the whole workspace
- `CHANGELOG.md` contains a non-empty section for the release tag
- author metadata uses `Weiming Zhai (wmzhai@gmail.com)`

## Verification

Run before a release:

```bash
pnpm install --frozen-lockfile
python3 tools/docs/generate-doc-site
cargo doc --workspace --no-deps
pnpm run build:website
for crate in alpaca-core alpaca-rest-http alpaca-data alpaca-trade alpaca-mock alpaca-time alpaca-option alpaca-facade; do
  cargo package --list --allow-dirty -p "$crate"
done
cargo publish --dry-run --locked --allow-dirty --no-verify -p alpaca-core
cargo publish --dry-run --locked --allow-dirty --no-verify -p alpaca-time
```

`cargo publish --dry-run` cannot fully preflight downstream crates until their
new dependency versions are visible on crates.io. The release workflow therefore
performs staged dry-runs inside the dependency-ordered publish loop.

## Minimal Patch Release Flow

Use this flow for the next patch release, for example `v0.24.8`.

1. Update the release metadata on `master`
   - bump `[workspace.package].version` in `Cargo.toml`
   - bump all in-workspace dependency version pins in crate `Cargo.toml` files
   - bump the website package version in `website/package.json`
   - update versioned install snippets in `docs/getting-started.md` and `docs/installation.md`
   - add a non-empty `## v0.24.8` section to `CHANGELOG.md`
   - ensure the three-layer crate list is consistent across `README.md`, `docs/`, generated docs metadata, and the website
   - regenerate docs metadata with `python3 tools/docs/generate-doc-site`

2. Run the local preflight

```bash
pnpm install --frozen-lockfile
python3 tools/docs/generate-doc-site
cargo doc --workspace --no-deps
pnpm run build:website
for crate in alpaca-core alpaca-rest-http alpaca-data alpaca-trade alpaca-mock alpaca-time alpaca-option alpaca-facade; do
  cargo package --list --allow-dirty -p "$crate"
done
cargo publish --dry-run --locked --allow-dirty --no-verify -p alpaca-core
cargo publish --dry-run --locked --allow-dirty --no-verify -p alpaca-time
```

3. Commit and push `master`

```bash
git add Cargo.toml crates docs website CHANGELOG.md .github/workflows/github-pages.yml memory AGENTS.md design.md
git commit -m "chore: prepare v0.24.8 release (v0.24.8)"
git push origin master
```

4. Create and push the tag

```bash
git tag v0.24.8
git push origin v0.24.8
```

5. Watch the release workflow

```bash
gh run list --workflow github-pages.yml --limit 5
gh run watch <run-id> --interval 10 --exit-status
```

6. Verify the published artifacts

```bash
gh release view v0.24.8 --json name,tagName,url --jq '.'
curl -fsS https://crates.io/api/v1/crates/alpaca-core/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-rest-http/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-data/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-trade/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-mock/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-time/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-option/0.24.8 >/dev/null
curl -fsS https://crates.io/api/v1/crates/alpaca-facade/0.24.8 >/dev/null
curl -I -L https://wmzhai.github.io/alpaca-rust/
```

## Publishing Order

Publish in dependency order:

1. `alpaca-core`
2. `alpaca-rest-http`
3. `alpaca-data`
4. `alpaca-trade`
5. `alpaca-mock`
6. `alpaca-time`
7. `alpaca-option`
8. `alpaca-facade`

Wait for crates.io index visibility before publishing each dependent crate.

## GitHub Automation

- Release automation is tag-only and runs from `.github/workflows/github-pages.yml`
- crates.io publication uses Trusted Publishing from GitHub Actions
- staged `cargo publish --dry-run --no-verify` checks run immediately before each real publish in dependency order
- GitHub Pages deploys from the same tag workflow
- GitHub Release creation runs from the same tag workflow
- First-time crates should be published manually once before relying on Trusted Publishing in CI

## Failure Recovery

- If the workflow fails before any crate is uploaded, fix the issue on `master`, bump to the next patch version, and tag that new version
- If the workflow already published some crates for a version but the tagged source is incorrect, fix `master`, bump to the next patch version, and tag that new version instead of reusing the partial release
- If the workflow file itself is broken on the tagged commit, do not reuse that tag; fix `master`, bump to the next patch version, and tag again
- If GitHub Pages deployment is blocked by environment rules, fix the `github-pages` environment and rerun the same workflow
- If the GitHub Release step fails after crates and Pages already succeeded, create or edit the GitHub Release for the same tag instead of republishing crates
