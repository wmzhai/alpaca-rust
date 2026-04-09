#![allow(dead_code)]

use alpaca_trade::{
    Error,
    orders::OrderStatus,
    positions::ClosePositionRequest,
};

use crate::target_support::TradeTestHarness;

pub(crate) async fn wait_for_order_status(
    harness: &TradeTestHarness,
    order_id: &str,
    expected_status: OrderStatus,
) -> alpaca_trade::orders::Order {
    for _attempt in 0..harness.poll_attempts() {
        let order = harness
            .trade_client()
            .orders()
            .get(order_id)
            .await
            .expect("order should remain readable");
        if order.status == expected_status {
            return order;
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    harness
        .trade_client()
        .orders()
        .get(order_id)
        .await
        .expect("order should remain readable")
}

pub(crate) async fn wait_for_position(
    harness: &TradeTestHarness,
    symbol: &str,
) -> alpaca_trade::positions::Position {
    for _attempt in 0..harness.poll_attempts() {
        if let Ok(position) = harness.trade_client().positions().get(symbol).await {
            return position;
        }
        tokio::time::sleep(harness.poll_interval()).await;
    }

    harness
        .trade_client()
        .positions()
        .get(symbol)
        .await
        .expect("position should become readable")
}

pub(crate) async fn wait_for_position_absent(harness: &TradeTestHarness, symbol: &str) {
    for _attempt in 0..harness.poll_attempts() {
        match harness.trade_client().positions().get(symbol).await {
            Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {
                return;
            }
            Err(other) => panic!("unexpected position lookup error: {other:?}"),
            Ok(_) => tokio::time::sleep(harness.poll_interval()).await,
        }
    }

    match harness.trade_client().positions().get(symbol).await {
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => {}
        other => panic!("position {symbol} should disappear, got {other:?}"),
    }
}

pub(crate) async fn ensure_symbol_flat(harness: &TradeTestHarness, symbol: &str) {
    let existing = match harness.trade_client().positions().get(symbol).await {
        Ok(position) => Some(position),
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => None,
        Err(other) => panic!("unexpected preflight position lookup error: {other:?}"),
    };
    if existing.is_none() {
        return;
    }

    let close = harness
        .trade_client()
        .positions()
        .close(symbol, ClosePositionRequest::default())
        .await
        .expect("preflight close position should submit");
    let _ = wait_for_order_status(harness, &close.id, OrderStatus::Filled).await;
    wait_for_position_absent(harness, symbol).await;
}
