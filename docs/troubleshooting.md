# Troubleshooting

## `MissingCredentials`

If `Client::from_env()` or `Client::builder().credentials_from_env()?` returns `MissingCredentials`, confirm the right environment variables are present.

Market data:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

Trading:

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`

## Wrong Base URL

If requests go to the wrong environment:

- `alpaca-data` reads `ALPACA_DATA_BASE_URL`
- `alpaca-trade` reads `ALPACA_TRADE_BASE_URL`
- `alpaca-trade` defaults to paper trading unless you switch to `.live()`

## `alpaca-mock` Starts But Orders Fail

Common causes:

- `ALPACA_DATA_API_KEY` / `ALPACA_DATA_SECRET_KEY` are not set for market-data-backed mock flows
- auth headers are missing on trading routes
- an HTTP fault was injected through `/admin/faults/http`

Check:

```bash
curl http://127.0.0.1:3847/health
curl http://127.0.0.1:3847/admin/state
```

## Docs Site Build Issues

Rebuild with:

```bash
python3 tools/docs/generate-doc-site
npm ci --prefix website
npm run build --prefix website
```

## Downstream `cargo publish --dry-run` Fails Before First Release

Before the first real crates.io publication, dependent workspace crates can fail dry-run because upstream workspace crates are not yet available on crates.io.

Expected publish order:

1. `alpaca-core`
2. `alpaca-rest-http`
3. `alpaca-data`
4. `alpaca-trade`
5. `alpaca-mock`

## Real API Tests Skip Or Fail Outside Market Conditions

Some live tests depend on paper trading state or current market conditions. Use the documented release-confidence subset that matches the current environment, and do not replace live verification with fake data branches.
