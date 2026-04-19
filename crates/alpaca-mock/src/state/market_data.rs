use rust_decimal::Decimal;

use alpaca_data::{
    Client,
    options::{
        SnapshotsRequest as OptionSnapshotsRequest, preferred_feed as preferred_option_feed,
    },
    stocks::{SnapshotsRequest as StockSnapshotsRequest, preferred_feed as preferred_stock_feed},
};

use super::MarketDataBridgeError;

pub const DEFAULT_STOCK_SYMBOL: &str = "SPY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentSnapshot {
    pub asset_class: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub previous_close: Option<Decimal>,
}

impl InstrumentSnapshot {
    pub fn equity(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_equity".to_owned(),
            bid,
            ask,
            previous_close: Some(mid_price(bid, ask)),
        }
    }

    pub fn option(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_option".to_owned(),
            bid,
            ask,
            previous_close: Some(mid_price(bid, ask)),
        }
    }

    pub fn mid_price(&self) -> Decimal {
        mid_price(self.bid, self.ask)
    }
}

#[derive(Debug, Clone)]
pub struct LiveMarketDataBridge {
    client: Client,
}

impl LiveMarketDataBridge {
    pub fn from_env() -> Result<Self, MarketDataBridgeError> {
        Ok(Self {
            client: Client::from_env()?,
        })
    }

    pub fn from_env_optional() -> Result<Option<Self>, MarketDataBridgeError> {
        match Client::from_env() {
            Ok(client) => Ok(Some(Self { client })),
            Err(alpaca_data::Error::MissingCredentials) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn instrument_snapshot(
        &self,
        symbol: &str,
    ) -> Result<InstrumentSnapshot, MarketDataBridgeError> {
        if looks_like_occ_option_symbol(symbol) {
            self.option_snapshot(symbol).await
        } else {
            self.equity_snapshot(symbol).await
        }
    }

    pub async fn equity_snapshot(
        &self,
        symbol: &str,
    ) -> Result<InstrumentSnapshot, MarketDataBridgeError> {
        let snapshot = self
            .client
            .stocks()
            .snapshots(StockSnapshotsRequest {
                symbols: vec![symbol.to_owned()],
                feed: Some(preferred_stock_feed(false)),
                currency: None,
            })
            .await?;
        let snapshot = snapshot
            .get(&alpaca_data::stocks::display_stock_symbol(symbol))
            .cloned()
            .ok_or_else(|| {
                MarketDataBridgeError::Unavailable(format!(
                    "stock snapshots response did not include {symbol}"
                ))
            })?;
        let quote = snapshot.latest_quote.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "stock snapshot for {symbol} did not include latest_quote"
            ))
        })?;
        let bid = quote.bp.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "stock snapshot for {symbol} did not include bid price"
            ))
        })?;
        let ask = quote.ap.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "stock snapshot for {symbol} did not include ask price"
            ))
        })?;

        Ok(InstrumentSnapshot {
            asset_class: "us_equity".to_owned(),
            bid,
            ask,
            previous_close: snapshot.prev_daily_bar.and_then(|bar| bar.c),
        })
    }

    pub async fn option_snapshot(
        &self,
        symbol: &str,
    ) -> Result<InstrumentSnapshot, MarketDataBridgeError> {
        let response = self
            .client
            .options()
            .snapshots(OptionSnapshotsRequest {
                symbols: vec![symbol.to_owned()],
                feed: Some(preferred_option_feed()),
                limit: Some(1),
                page_token: None,
            })
            .await?;
        let snapshot = response.snapshots.get(symbol).cloned().ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "option snapshot response did not include {symbol}"
            ))
        })?;
        let quote = snapshot.latest_quote.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "option snapshot for {symbol} did not include latest_quote"
            ))
        })?;
        let bid = quote.bp.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "option snapshot for {symbol} did not include bid price"
            ))
        })?;
        let ask = quote.ap.ok_or_else(|| {
            MarketDataBridgeError::Unavailable(format!(
                "option snapshot for {symbol} did not include ask price"
            ))
        })?;

        Ok(InstrumentSnapshot {
            asset_class: "us_option".to_owned(),
            bid,
            ask,
            previous_close: snapshot.prev_daily_bar.and_then(|bar| bar.c),
        })
    }
}

pub fn mid_price(bid: Decimal, ask: Decimal) -> Decimal {
    ((bid + ask) / Decimal::new(2, 0)).round_dp(2)
}

fn looks_like_occ_option_symbol(symbol: &str) -> bool {
    let symbol = symbol.trim();
    if symbol.len() <= 15 {
        return false;
    }

    let suffix = &symbol[symbol.len() - 15..];
    suffix[..6].chars().all(|ch| ch.is_ascii_digit())
        && matches!(&suffix[6..7], "C" | "P")
        && suffix[7..].chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use alpaca_data::{options::OptionsFeed, stocks::DataFeed};

    use super::mid_price;
    use alpaca_data::{options::preferred_feed as preferred_option_feed, stocks::preferred_feed};

    #[test]
    fn market_data_bridge_uses_premium_provider_feeds() {
        assert_eq!(preferred_option_feed(), OptionsFeed::Opra);
        assert_eq!(preferred_feed(false), DataFeed::Sip);
    }

    #[test]
    fn mid_price_rounds_to_two_decimals() {
        assert_eq!(
            mid_price(
                rust_decimal::Decimal::new(1064, 2),
                rust_decimal::Decimal::new(1110, 2)
            ),
            rust_decimal::Decimal::new(1087, 2)
        );
    }
}
