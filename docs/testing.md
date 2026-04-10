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

Run the currently implemented market-data integration suite:

```bash
cargo test -p alpaca-data --tests -- --nocapture
```

Current market-data test files:

- `crates/alpaca-data/tests/stocks_real_api.rs`
- `crates/alpaca-data/tests/options_real_api.rs`
- `crates/alpaca-data/tests/news_real_api.rs`
- `crates/alpaca-data/tests/corporate_actions_real_api.rs`

## Trading Tests

Run the trading integration suite:

```bash
cargo test -p alpaca-trade --tests -- --nocapture
```

Notable test lanes:

- real API reads: account, assets, calendar, clock, portfolio history, watchlists, options contracts
- write-path and contract tests: orders, positions, activities
- trade mainline: `crates/alpaca-trade/tests/mainline_api.rs`

## Mock Tests

Run the mock crate tests:

```bash
cargo test -p alpaca-mock -- --nocapture
```

Current mock-focused coverage:

- route coverage: `crates/alpaca-mock/tests/app_routes.rs`
- market-data bridge coverage: `crates/alpaca-mock/tests/market_data_real_api.rs`
- contract-style mock verification in `alpaca-trade`:
  - `crates/alpaca-trade/tests/orders_mock_contract.rs`
  - `crates/alpaca-trade/tests/positions_mock_contract.rs`

## Release-Confidence Commands

```bash
cargo fmt --check
cargo check --workspace
cargo test --doc
python3 tools/docs/generate-doc-site
npm run build --prefix website
```

## Scope Notes

- mock verification does not replace official live API verification
- market-data-backed mock flows still depend on real `alpaca-data` calls
- tests should not silently fall back to invented market data
