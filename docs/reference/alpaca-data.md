# alpaca-data

Async market data client crate.

Resource accessors:

- `stocks()`
- `options()`
- `news()`
- `corporate_actions()`

Mirror coverage in the current release line:

- stocks: historical bars/quotes/trades, latest bars/quotes/trades, snapshots, auctions, condition codes, exchange codes
- options: bars, trades, latest quotes/trades, snapshots, chains, condition codes, exchange codes
- news: list
- corporate actions: list

Convenience coverage:

- `*_all` pagination aggregators for all currently paginated adopted endpoints
- `options.snapshots_all(...)` also absorbs Alpaca's 100-contract batch limit internally
- `alpaca_data::options::underlying_symbol(...)` canonicalizes option underlying / OCC-root input such as `BRK.B -> BRKB`
- `alpaca_data::stocks::display_symbol(...)` restores supported dotted stock display symbols such as `BRKB -> BRK.B`
- stock latest/snapshot reads also stay on canonical batch request types, even for single symbols
- `alpaca_data::stocks::Snapshot::{timestamp,price,bid_price,ask_price,session_open,session_high,session_low,session_close,previous_close,session_volume}` exposes provider-safe quote/session value selection
- `alpaca_data::options::Snapshot::{timestamp,bid_price,ask_price,last_price,mark_price}` exposes provider-safe snapshot value selection
- `alpaca_data::options::ordered_snapshots(...)` returns stable contract ordering
- `alpaca_data::corporate_actions::ListRequest` absorbs provider-safe stock symbol normalization such as `brk/b -> BRK.B`

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
