use std::str::FromStr;

use redis::Commands;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::positions::{PositionBook, SignedPosition};

const KEY_PREFIX: &str = "alpaca-mock:positions:";

#[derive(Clone)]
pub(crate) struct PositionRedis {
    client: redis::Client,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct StoredPosition {
    symbol: String,
    qty: String,
    avg_entry_price: String,
}

impl std::fmt::Debug for PositionRedis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PositionRedis")
    }
}

impl PositionRedis {
    pub(crate) fn connect(url: &str) -> Result<Self, String> {
        let client = redis::Client::open(url).map_err(|error| error.to_string())?;
        let mut connection = client
            .get_connection()
            .map_err(|error| error.to_string())?;
        redis::cmd("PING")
            .query::<String>(&mut connection)
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    pub(crate) fn from_env() -> Result<Option<Self>, String> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned());
        if url.trim().is_empty() {
            return Ok(None);
        }
        Self::connect(url.trim()).map(Some)
    }

    pub(crate) fn load_all(&self) -> Result<Vec<(String, Vec<SignedPosition>)>, String> {
        let mut connection = self.connection()?;
        let keys = redis::cmd("KEYS")
            .arg(format!("{KEY_PREFIX}*"))
            .query::<Vec<String>>(&mut connection)
            .map_err(|error| error.to_string())?;
        let mut loaded = Vec::new();
        for key in keys {
            let Some(api_key) = key.strip_prefix(KEY_PREFIX) else {
                continue;
            };
            let payload: String = connection.get(&key).map_err(|error| error.to_string())?;
            loaded.push((api_key.to_owned(), decode_positions(&payload)?));
        }
        Ok(loaded)
    }

    pub(crate) fn save(&self, api_key: &str, positions: &[SignedPosition]) -> Result<(), String> {
        let mut connection = self.connection()?;
        let key = format!("{KEY_PREFIX}{api_key}");
        if positions.is_empty() {
            let _: () = connection.del(&key).map_err(|error| error.to_string())?;
            return Ok(());
        }
        let payload = serde_json::to_string(&encode_positions(positions)).map_err(|error| error.to_string())?;
        let _: () = connection.set(&key, payload).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), String> {
        let mut connection = self.connection()?;
        let keys = redis::cmd("KEYS")
            .arg(format!("{KEY_PREFIX}*"))
            .query::<Vec<String>>(&mut connection)
            .map_err(|error| error.to_string())?;
        if keys.is_empty() {
            return Ok(());
        }
        let _: () = connection.del(keys).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn connection(&self) -> Result<redis::Connection, String> {
        self.client.get_connection().map_err(|error| error.to_string())
    }
}

pub(crate) fn restore_positions(book: &mut PositionBook, positions: Vec<SignedPosition>) {
    book.replace_signed(positions);
}

fn encode_positions(positions: &[SignedPosition]) -> Vec<StoredPosition> {
    positions
        .iter()
        .map(|position| StoredPosition {
            symbol: position.symbol.clone(),
            qty: position.qty.normalize().to_string(),
            avg_entry_price: position.avg_entry_price.normalize().to_string(),
        })
        .collect()
}

fn decode_positions(payload: &str) -> Result<Vec<SignedPosition>, String> {
    let stored: Vec<StoredPosition> = serde_json::from_str(payload).map_err(|error| error.to_string())?;
    stored
        .into_iter()
        .map(|position| {
            Ok(SignedPosition {
                symbol: position.symbol,
                qty: Decimal::from_str(&position.qty).map_err(|error| error.to_string())?,
                avg_entry_price: Decimal::from_str(&position.avg_entry_price).map_err(|error| error.to_string())?,
            })
        })
        .collect()
}
