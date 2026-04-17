#![allow(dead_code)]

use alpaca_trade::{
    Error,
    orders::{ListRequest, OrderStatus, QueryOrderStatus, WaitFor},
    positions::ClosePositionRequest,
};

use crate::target_support::TradeTestHarness;

pub(crate) async fn wait_for_order_status(
    harness: &TradeTestHarness,
    order_id: &str,
    expected_status: OrderStatus,
) -> alpaca_trade::orders::Order {
    harness
        .trade_client()
        .orders()
        .wait_for(order_id, WaitFor::Exact(expected_status))
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
    cancel_open_orders_for_symbol(harness, symbol).await;

    let existing = match harness.trade_client().positions().get(symbol).await {
        Ok(position) => Some(position),
        Err(Error::Http(error)) if error.meta().map(|meta| meta.status()) == Some(404) => None,
        Err(other) => panic!("unexpected preflight position lookup error: {other:?}"),
    };
    if existing.is_none() {
        return;
    }

    for attempt in 0..harness.poll_attempts() {
        match harness
            .trade_client()
            .positions()
            .close(symbol, ClosePositionRequest::default())
            .await
        {
            Ok(close) => {
                let _ = wait_for_order_status(harness, &close.id, OrderStatus::Filled).await;
                wait_for_position_absent(harness, symbol).await;
                return;
            }
            Err(Error::Http(error))
                if error.meta().map(|meta| meta.status()) == Some(403)
                    && error
                        .meta()
                        .and_then(|meta| meta.body_snippet())
                        .is_some_and(|body| body.contains("\"held_for_orders\"")) =>
            {
                if attempt + 1 == harness.poll_attempts() {
                    panic!("preflight close position should submit: {error:?}");
                }
                tokio::time::sleep(harness.poll_interval()).await;
            }
            Err(other) => panic!("preflight close position should submit: {other:?}"),
        }
    }
}

async fn cancel_open_orders_for_symbol(harness: &TradeTestHarness, symbol: &str) {
    let open_orders = harness
        .trade_client()
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(100),
            symbols: Some(vec![symbol.to_owned()]),
            ..ListRequest::default()
        })
        .await
        .expect("preflight open orders should remain readable");

    for order in open_orders {
        let resolved = harness
            .trade_client()
            .orders()
            .cancel_resolved(&order.id)
            .await
            .expect("preflight open order cancel should submit");
        assert!(
            is_terminal_status(&resolved.order.status),
            "preflight open order cancel should end in a terminal state: order={}, status={:?}",
            order.id,
            resolved.order.status
        );
    }
}

fn is_terminal_status(status: &OrderStatus) -> bool {
    status.is_terminal() || *status == OrderStatus::Replaced
}
