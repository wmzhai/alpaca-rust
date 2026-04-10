# alpaca-data

Async market data client crate.

Resource accessors:

- `stocks()`
- `options()`
- `news()`
- `corporate_actions()`

Mirror coverage in the current release line:

- stocks: historical bars/quotes/trades, single-symbol historical bars/quotes/trades, latest bars/quotes/trades, snapshots, auctions, condition codes, exchange codes
- options: bars, trades, latest quotes/trades, snapshots, chains, condition codes, exchange codes
- news: list
- corporate actions: list

Convenience coverage:

- `*_all` pagination aggregators for all currently paginated adopted endpoints

Environment variables:

- `ALPACA_DATA_API_KEY`
- `ALPACA_DATA_SECRET_KEY`
- `ALPACA_DATA_BASE_URL`

See also:

- [Stocks](./stocks.md)
- [Options Market Data](./options-data.md)
- [News](./news.md)
- [Corporate Actions](./corporate-actions.md)
- [Market Data API Coverage](../api-coverage/market-data.md)
- [docs.rs/alpaca-data](https://docs.rs/alpaca-data)

Not implemented:

- crypto
- forex
- fixed income
- logos
- screener
- websocket and streaming APIs
