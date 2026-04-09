#[path = "../../../tests/support/live/mod.rs"]
mod live_support;
#[path = "support/orders.rs"]
mod order_support;

use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_data::Client as DataClient;
use alpaca_mock::{
    DEFAULT_STOCK_SYMBOL, LiveMarketDataBridge, MockServerState, spawn_test_server_with_state,
};
use alpaca_trade::{
    Client,
    orders::{
        CreateRequest, ListRequest, OrderClass, OrderSide, OrderStatus, OrderType,
        QueryOrderStatus, ReplaceRequest, TimeInForce,
    },
};
use rust_decimal::Decimal;

use live_support::{AlpacaService, LiveTestEnv};
use order_support::{
    discover_mleg_call_spread, discover_mleg_iron_condor, discover_mleg_put_spread,
};

#[tokio::test]
async fn orders_mock_supports_basic_lifecycle() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock orders test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let market_bridge = LiveMarketDataBridge::new(data_client);
    let price_context = market_bridge
        .equity_snapshot(DEFAULT_STOCK_SYMBOL)
        .await
        .expect("equity snapshot should load for mock pricing");
    let non_marketable_buy_price = (price_context.mid_price() * Decimal::new(5, 1)).round_dp(2);
    let state = MockServerState::new().with_market_data_bridge(market_bridge);
    let server = spawn_test_server_with_state(state).await;
    let client_order_id = format!(
        "phase12-mock-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(DEFAULT_STOCK_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(non_marketable_buy_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock order create should succeed");
    assert_eq!(created.symbol, DEFAULT_STOCK_SYMBOL);
    assert_eq!(created.status, OrderStatus::New);

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock order get should succeed");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("mock client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let replaced_client_order_id = format!("{client_order_id}-replaced");
    let replaced = client
        .orders()
        .replace(
            &created.id,
            ReplaceRequest {
                qty: None,
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some((non_marketable_buy_price * Decimal::new(9, 1)).round_dp(2)),
                stop_price: None,
                trail: None,
                client_order_id: Some(replaced_client_order_id.clone()),
            },
        )
        .await
        .expect("mock order replace should succeed");
    assert_ne!(replaced.id, created.id);
    assert_eq!(replaced.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(replaced.client_order_id, replaced_client_order_id);
    assert_eq!(replaced.status, OrderStatus::New);

    let replaced_source = client
        .orders()
        .get(&created.id)
        .await
        .expect("replaced source order should remain readable");
    assert_eq!(replaced_source.status, OrderStatus::Replaced);
    assert_eq!(
        replaced_source.replaced_by.as_deref(),
        Some(replaced.id.as_str())
    );

    let open_orders = client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            ..ListRequest::default()
        })
        .await
        .expect("mock order list should succeed");
    assert!(open_orders.iter().any(|order| order.id == replaced.id));

    client
        .orders()
        .cancel(&replaced.id)
        .await
        .expect("mock order cancel should succeed");

    let canceled = client
        .orders()
        .get(&replaced.id)
        .await
        .expect("mock canceled order should still be readable");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}

#[tokio::test]
async fn orders_mock_supports_mleg_lifecycle_with_replace_and_cancel() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock mleg orders test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let spread = discover_mleg_call_spread(&data_client, DEFAULT_STOCK_SYMBOL)
        .await
        .expect("dynamic call spread should be discoverable");
    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client));
    let server = spawn_test_server_with_state(state).await;
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    let client_order_id = format!(
        "phase20-mock-mleg-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_millis()
    );
    let created = client
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(spread.non_marketable_limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some(client_order_id.clone()),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("mock multi-leg order create should succeed");
    assert_eq!(created.order_class, OrderClass::Mleg);
    assert_eq!(created.status, OrderStatus::New);
    assert_mleg_parent_shape(&created, spread.legs.len());

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock multi-leg get should succeed");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id(&client_order_id)
        .await
        .expect("mock multi-leg client_order_id lookup should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let created_legs = created
        .legs
        .clone()
        .expect("created mleg should include legs");
    let replaced = client
        .orders()
        .replace(
            &created.id,
            ReplaceRequest {
                qty: None,
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(spread.more_conservative_limit_price),
                stop_price: None,
                trail: None,
                client_order_id: Some(format!("{client_order_id}-replaced")),
            },
        )
        .await
        .expect("mock multi-leg replace should succeed");
    assert_eq!(replaced.order_class, OrderClass::Mleg);
    assert_eq!(replaced.status, OrderStatus::New);
    assert_eq!(replaced.replaces.as_deref(), Some(created.id.as_str()));
    assert_mleg_parent_shape(&replaced, spread.legs.len());

    let replaced_source = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock replaced source order should remain readable");
    assert_eq!(replaced_source.status, OrderStatus::Replaced);
    assert_eq!(
        replaced_source.replaced_by.as_deref(),
        Some(replaced.id.as_str())
    );

    let replaced_legs = replaced
        .legs
        .clone()
        .expect("replacement should keep nested legs");
    assert_eq!(replaced_legs.len(), created_legs.len());
    for (old_leg, new_leg) in created_legs.iter().zip(replaced_legs.iter()) {
        assert_ne!(new_leg.id, old_leg.id);
        assert_eq!(new_leg.replaces.as_deref(), Some(old_leg.id.as_str()));
        assert_eq!(new_leg.status, OrderStatus::New);
    }

    client
        .orders()
        .cancel(&replaced.id)
        .await
        .expect("mock multi-leg cancel should succeed");

    let canceled = client
        .orders()
        .get(&replaced.id)
        .await
        .expect("mock canceled multi-leg should remain readable");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    let canceled_legs = canceled
        .legs
        .expect("canceled mleg should keep nested legs");
    assert!(
        canceled_legs
            .iter()
            .all(|leg| leg.status == OrderStatus::Canceled)
    );
}

#[tokio::test]
async fn orders_mock_cancel_all_cancels_stock_and_mleg_orders() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock cancel_all orders test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let market_bridge = LiveMarketDataBridge::new(data_client.clone());
    let price_context = market_bridge
        .equity_snapshot(DEFAULT_STOCK_SYMBOL)
        .await
        .expect("equity snapshot should load for mock pricing");
    let spread = discover_mleg_call_spread(&data_client, DEFAULT_STOCK_SYMBOL)
        .await
        .expect("dynamic call spread should be discoverable");
    let state = MockServerState::new().with_market_data_bridge(market_bridge);
    let server = spawn_test_server_with_state(state).await;
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    let stock_created = client
        .orders()
        .create(CreateRequest {
            symbol: Some(DEFAULT_STOCK_SYMBOL.to_owned()),
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some((price_context.mid_price() * Decimal::new(5, 1)).round_dp(2)),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some("phase20-mock-cancel-all-stock".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock stock order create should succeed");
    let mleg_created = client
        .orders()
        .create(CreateRequest {
            symbol: None,
            qty: Some(Decimal::ONE),
            notional: None,
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(spread.non_marketable_limit_price),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            extended_hours: Some(false),
            client_order_id: Some("phase20-mock-cancel-all-mleg".to_owned()),
            order_class: Some(OrderClass::Mleg),
            take_profit: None,
            stop_loss: None,
            legs: Some(spread.legs.clone()),
            position_intent: None,
        })
        .await
        .expect("mock multi-leg order create should succeed");

    let canceled = client
        .orders()
        .cancel_all()
        .await
        .expect("mock cancel_all should succeed");
    assert!(canceled.iter().any(|result| result.id == stock_created.id));
    assert!(canceled.iter().any(|result| result.id == mleg_created.id));

    let mleg_cancel_body = canceled
        .iter()
        .find(|result| result.id == mleg_created.id)
        .and_then(|result| result.body.clone())
        .expect("mock cancel_all should return the canceled mleg order");
    assert_eq!(mleg_cancel_body.status, OrderStatus::Canceled);
    assert!(
        mleg_cancel_body
            .legs
            .is_some_and(|legs| { legs.iter().all(|leg| leg.status == OrderStatus::Canceled) })
    );
}

#[tokio::test]
async fn orders_mock_marketable_multi_leg_orders_fill_for_spreads_and_condors() {
    let env = LiveTestEnv::load().expect("live test environment should load");
    if let Some(reason) = env.skip_reason_for_service(AlpacaService::Data) {
        eprintln!("skipping mock marketable mleg orders test: {reason}");
        return;
    }

    let data_service = env.data().expect("data config should exist");
    let data_client = DataClient::builder()
        .credentials(data_service.credentials().clone())
        .base_url(data_service.base_url().clone())
        .build()
        .expect("data client should build from live service config");
    let put_spread = discover_mleg_put_spread(&data_client, DEFAULT_STOCK_SYMBOL)
        .await
        .expect("dynamic put spread should be discoverable");
    let maybe_iron_condor = discover_mleg_iron_condor(&data_client, DEFAULT_STOCK_SYMBOL).await;
    let state =
        MockServerState::new().with_market_data_bridge(LiveMarketDataBridge::new(data_client));
    let server = spawn_test_server_with_state(state).await;
    let client = Client::builder()
        .api_key("mock-key")
        .secret_key("mock-secret")
        .base_url_str(&server.base_url)
        .expect("mock base url should parse")
        .build()
        .expect("mock trade client should build");

    let mut strategies = vec![("put-spread", put_spread)];
    match maybe_iron_condor {
        Ok(iron_condor) => strategies.push(("iron-condor", iron_condor)),
        Err(reason) => eprintln!("skipping mock iron condor subcase: {reason}"),
    }

    for (name, strategy) in strategies {
        let filled = client
            .orders()
            .create(CreateRequest {
                symbol: None,
                qty: Some(Decimal::ONE),
                notional: None,
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(strategy.marketable_limit_price),
                stop_price: None,
                trail_price: None,
                trail_percent: None,
                extended_hours: Some(false),
                client_order_id: Some(format!("phase20-mock-{name}")),
                order_class: Some(OrderClass::Mleg),
                take_profit: None,
                stop_loss: None,
                legs: Some(strategy.legs.clone()),
                position_intent: None,
            })
            .await
            .expect("mock marketable multi-leg order create should succeed");
        assert_eq!(filled.status, OrderStatus::Filled);
        assert!(filled.filled_avg_price.is_some());
        let filled_legs = filled.legs.expect("filled mleg should keep nested legs");
        assert_eq!(filled_legs.len(), strategy.legs.len());
        assert!(
            filled_legs
                .iter()
                .all(|leg| leg.status == OrderStatus::Filled)
        );
        assert!(filled_legs.iter().all(|leg| leg.filled_avg_price.is_some()));
    }
}

fn assert_mleg_parent_shape(order: &alpaca_trade::orders::Order, expected_leg_count: usize) {
    assert_eq!(order.order_class, OrderClass::Mleg);
    assert_eq!(order.symbol, "");
    assert_eq!(order.asset_class, "");
    assert_eq!(order.side, OrderSide::Unspecified);
    assert_eq!(order.position_intent, None);
    let legs = order
        .legs
        .as_ref()
        .expect("mleg parent should include legs");
    assert_eq!(legs.len(), expected_leg_count);
    assert!(legs.iter().all(|leg| leg.order_class == OrderClass::Mleg));
    assert!(legs.iter().all(|leg| leg.asset_class == "us_option"));
    assert!(legs.iter().all(|leg| leg.limit_price.is_none()));
    assert!(legs.iter().all(|leg| leg.ratio_qty.is_some()));
}
