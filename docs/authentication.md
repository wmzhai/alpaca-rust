# Authentication

`alpaca-rust` keeps market data and trading credentials separate.

## Market Data

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- optional `ALPACA_DATA_BASE_URL`
- legacy base URL fallback: `APCA_API_DATA_URL`

## Trading

- `ALPACA_TRADE_API_KEY`
- `ALPACA_TRADE_SECRET_KEY`
- optional `ALPACA_TRADE_BASE_URL`

## Mock Server

`alpaca-mock` does not require Alpaca trading credentials for its own listening socket, but market-data-backed flows still use:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`

Optional runtime binding:

- `ALPACA_MOCK_LISTEN_ADDR`

## Builder Helpers

- `alpaca_data::Client::from_env()`
- `alpaca_trade::Client::from_env()`
- `Client::builder().credentials_from_env()?.base_url_from_env()?`
