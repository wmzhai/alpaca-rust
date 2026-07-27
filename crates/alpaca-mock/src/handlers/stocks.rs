use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use alpaca_data::stocks::{Bar, Quote, Snapshot, Trade};

use crate::auth::MockHttpError;
use crate::state::MockServerState;

#[derive(Debug, Serialize)]
pub(crate) struct StockSnapshotResponse {
    symbol: String,
    #[serde(flatten)]
    snapshot: Snapshot,
}

pub(crate) async fn stocks_snapshot(
    State(state): State<MockServerState>,
    Path(symbol): Path<String>,
) -> Result<Json<StockSnapshotResponse>, MockHttpError> {
    let (symbol, market) = state.runtime_stock_snapshot(&symbol)?;
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let price = market.mid_price();
    let previous_close = market.previous_close.map(|close| Bar {
        c: Some(close),
        ..Bar::default()
    });

    Ok(Json(StockSnapshotResponse {
        symbol,
        snapshot: Snapshot {
            latest_trade: Some(Trade {
                t: Some(timestamp.clone()),
                p: Some(price),
                ..Trade::default()
            }),
            latest_quote: Some(Quote {
                t: Some(timestamp),
                bp: Some(market.bid),
                ap: Some(market.ask),
                ..Quote::default()
            }),
            prev_daily_bar: previous_close,
            ..Snapshot::default()
        },
    }))
}
