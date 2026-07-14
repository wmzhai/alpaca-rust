# Testing Guide

`alpaca-rust` uses a live-first verification model for Alpaca-facing behavior.

## Data Credentials

Market-data tests use:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

Trading tests use:

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`

## Market Data Tests

Run the strict market-data network suite:

```bash
cargo test -p alpaca-data --tests -- --nocapture
```

Current market-data test files:

- `crates/alpaca-data/tests/stocks_real_api.rs`
- `crates/alpaca-data/tests/corporate_actions_real_api.rs`

Every retained test sends at least one request to the canonical Alpaca Data
API. Missing credentials fail the test; there are no unit, fixture, recorder,
ignored, or silent-skip tests in `alpaca-data`.

## Trading Tests

Run the shared Trading network suite against one explicit target:

```bash
cargo test -p alpaca-trade --test trading_contract_api -- --nocapture --test-threads=1
```

`T127_TRADING_TARGET` must be `paper` or `mock`. Paper requires the canonical
`https://paper-api.alpaca.markets` base URL and Paper credentials. Mock requires
a loopback URL for a separately running `alpaca-mock` process. Missing target,
base URL, key, or secret fails immediately.

All 28 retained tests live in
`crates/alpaca-trade/tests/trading_contract_api.rs`. Each test sends HTTP
requests through the public `alpaca-trade` client and verifies observed method,
path, status, and request ID. The crate has no unit, serde-only, algorithm-only,
fixture, in-process router, or mock-only tests.

`optionDoNotExercise` is the sole pending Trading operation. Paper accepts that
instruction only for a long option position on its expiration day. Run its
corrected exact Paper scenario only with a clean account so state assertions and
cleanup can complete; raw Paper and mock `200` responses alone do not establish
closure.

## Mock Tests

Run the mock crate tests:

```bash
cargo test -p alpaca-mock -- --nocapture
```

Current mock-focused coverage:

- route coverage: `crates/alpaca-mock/tests/app_routes.rs`
- market-data bridge coverage: `crates/alpaca-mock/tests/market_data_real_api.rs`
- Trading contract verification: run `trading_contract_api` against the
  separately running mock HTTP service.

## Release-Confidence Commands

```bash
cargo fmt --check
cargo check --workspace
python3 tools/docs/generate-doc-site
pnpm run build:website
```

## Scope Notes

- mock verification does not replace official live API verification
- market-data-backed mock flows still depend on real `alpaca-data` calls
- `alpaca-data` and `alpaca-trade` tests must always cross their public HTTP API
  boundary; local-only tests are intentionally not retained
- tests must not silently skip or fall back to invented market data
