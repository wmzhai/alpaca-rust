use std::collections::HashSet;

use alpaca_mock::RuntimeStockPriceResponse;
use alpaca_trade::Client;
use alpaca_trade::activities::ListRequest;
use alpaca_trade::orders::{
    CreateRequest, GetRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType, TimeInForce,
};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use serde_json::json;

const API_KEY: &str = "runtime-stock-price-key";
const SECRET_KEY: &str = "runtime-stock-price-secret";

#[tokio::test]
async fn runtime_stock_price_fills_existing_limit_orders_through_http_once() {
    let server = alpaca_mock::spawn_test_server().await;
    let http = reqwest::Client::new();
    let trading = Client::builder()
        .api_key(API_KEY)
        .secret_key(SECRET_KEY)
        .base_url_str(&server.base_url)
        .expect("mock base URL should be accepted")
        .build()
        .expect("trading client should build");

    let initial_gld = set_stock_price(&http, &server.base_url, "gld", "100.004").await;
    assert_eq!(initial_gld.symbol, "GLD");
    assert_eq!(initial_gld.price, Decimal::new(10_000, 2));
    assert!(initial_gld.filled_order_ids.is_empty());
    let initial_spy = set_stock_price(&http, &server.base_url, "SPY", "100.00").await;
    assert!(initial_spy.filled_order_ids.is_empty());

    let buy_two = create_limit_order(
        &trading,
        "GLD",
        Decimal::new(2, 0),
        OrderSide::Buy,
        Decimal::new(99, 0),
        "runtime-buy-two",
    )
    .await;
    let buy_one = create_limit_order(
        &trading,
        "GLD",
        Decimal::ONE,
        OrderSide::Buy,
        Decimal::new(99, 0),
        "runtime-buy-one",
    )
    .await;
    let sell_one = create_limit_order(
        &trading,
        "GLD",
        Decimal::ONE,
        OrderSide::Sell,
        Decimal::new(101, 0),
        "runtime-sell-one",
    )
    .await;
    let unrelated_spy = create_limit_order(
        &trading,
        "SPY",
        Decimal::ONE,
        OrderSide::Buy,
        Decimal::new(99, 0),
        "runtime-unrelated-spy",
    )
    .await;
    let already_filled = create_limit_order(
        &trading,
        "GLD",
        Decimal::ONE,
        OrderSide::Buy,
        Decimal::new(100, 0),
        "runtime-already-filled",
    )
    .await;

    for order in [&buy_two, &buy_one, &sell_one, &unrelated_spy] {
        assert_eq!(order.status, OrderStatus::New);
        assert_eq!(order.filled_qty, Decimal::ZERO);
    }
    assert_eq!(already_filled.status, OrderStatus::Filled);
    assert_eq!(already_filled.filled_avg_price, Some(Decimal::new(100, 0)));

    let crossed_buys = set_stock_price(&http, &server.base_url, "GLD", "99.00").await;
    let mut expected_buy_ids = vec![buy_two.id.clone(), buy_one.id.clone()];
    expected_buy_ids.sort();
    assert_eq!(crossed_buys.filled_order_ids, expected_buy_ids);

    for order in [&buy_two, &buy_one] {
        let filled = get_order(&trading, &order.id).await;
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(
            filled.filled_qty,
            order.qty.expect("test order should have qty")
        );
        assert_eq!(filled.filled_avg_price, Some(Decimal::new(99, 0)));
    }
    assert_eq!(
        get_order(&trading, &sell_one.id).await.status,
        OrderStatus::New
    );
    assert_eq!(
        get_order(&trading, &unrelated_spy.id).await.status,
        OrderStatus::New
    );
    assert_eq!(
        get_order(&trading, &already_filled.id)
            .await
            .filled_avg_price,
        Some(Decimal::new(100, 0))
    );

    let account_after_buys = trading
        .account()
        .get()
        .await
        .expect("mock account should load");
    assert_eq!(account_after_buys.cash, Some(Decimal::new(999_603, 0)));
    let position_after_buys = trading
        .positions()
        .get("GLD")
        .await
        .expect("GLD position should exist");
    assert_eq!(position_after_buys.qty, Decimal::new(4, 0));
    assert_eq!(position_after_buys.avg_entry_price, Decimal::new(9_925, 2));

    let fills_after_buys = trading
        .activities()
        .list(ListRequest::for_types(&["FILL"], None))
        .await
        .expect("fill activities should load");
    assert_eq!(fills_after_buys.len(), 3);
    assert_fill_ids(
        &fills_after_buys,
        [&already_filled.id, &buy_two.id, &buy_one.id],
    );

    let repeated = set_stock_price(&http, &server.base_url, "GLD", "99.00").await;
    assert!(repeated.filled_order_ids.is_empty());
    assert_eq!(
        trading
            .account()
            .get()
            .await
            .expect("mock account should reload")
            .cash,
        account_after_buys.cash
    );
    assert_eq!(
        trading
            .activities()
            .list(ListRequest::for_types(&["FILL"], None))
            .await
            .expect("fill activities should reload")
            .len(),
        fills_after_buys.len()
    );

    let crossed_sell = set_stock_price(&http, &server.base_url, "GLD", "101.00").await;
    assert_eq!(crossed_sell.filled_order_ids, vec![sell_one.id.clone()]);
    let filled_sell = get_order(&trading, &sell_one.id).await;
    assert_eq!(filled_sell.status, OrderStatus::Filled);
    assert_eq!(filled_sell.filled_avg_price, Some(Decimal::new(101, 0)));
    assert_eq!(
        trading
            .account()
            .get()
            .await
            .expect("mock account should load after sell")
            .cash,
        Some(Decimal::new(999_704, 0))
    );
    assert_eq!(
        trading
            .positions()
            .get("GLD")
            .await
            .expect("GLD position should remain")
            .qty,
        Decimal::new(3, 0)
    );
    let fills_after_sell = trading
        .activities()
        .list(ListRequest::for_types(&["FILL"], None))
        .await
        .expect("fill activities should load after sell");
    assert_eq!(fills_after_sell.len(), 4);
    assert_fill_ids(
        &fills_after_sell,
        [&already_filled.id, &buy_two.id, &buy_one.id, &sell_one.id],
    );
    assert_eq!(
        get_order(&trading, &unrelated_spy.id).await.status,
        OrderStatus::New
    );

    http.post(format!("{}/admin/reset", server.base_url))
        .send()
        .await
        .expect("admin reset request should succeed")
        .error_for_status()
        .expect("admin reset should return success");
    let post_reset_order = http
        .post(format!("{}/v2/orders", server.base_url))
        .header("apca-api-key-id", API_KEY)
        .header("apca-api-secret-key", SECRET_KEY)
        .json(&limit_order_json("GLD", "1", "buy", "99", "post-reset"))
        .send()
        .await
        .expect("post-reset order request should complete");
    assert_eq!(post_reset_order.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn set_stock_price(
    http: &reqwest::Client,
    base_url: &str,
    symbol: &str,
    price: &str,
) -> RuntimeStockPriceResponse {
    let body: serde_json::Value = http
        .post(format!("{base_url}/admin/market-data/stocks/{symbol}"))
        .json(&json!({ "price": price }))
        .send()
        .await
        .expect("runtime stock price request should succeed")
        .error_for_status()
        .expect("runtime stock price should return success")
        .json()
        .await
        .expect("runtime stock price response should be valid JSON");
    assert!(body["price"].is_string());
    serde_json::from_value(body).expect("runtime stock price response should deserialize")
}

async fn create_limit_order(
    trading: &Client,
    symbol: &str,
    qty: Decimal,
    side: OrderSide,
    limit_price: Decimal,
    client_order_id: &str,
) -> Order {
    trading
        .orders()
        .create(CreateRequest {
            symbol: Some(symbol.to_owned()),
            qty: Some(qty),
            side: Some(side),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Gtc),
            limit_price: Some(limit_price),
            extended_hours: Some(true),
            client_order_id: Some(client_order_id.to_owned()),
            order_class: Some(OrderClass::Simple),
            ..CreateRequest::default()
        })
        .await
        .expect("mock limit order should be created")
}

async fn get_order(trading: &Client, order_id: &str) -> Order {
    trading
        .orders()
        .get(order_id, GetRequest::default())
        .await
        .expect("mock order should load")
}

fn assert_fill_ids<'a>(
    activities: &[alpaca_trade::activities::Activity],
    expected: impl IntoIterator<Item = &'a String>,
) {
    let actual = activities
        .iter()
        .filter_map(|activity| activity.order_id.clone())
        .collect::<HashSet<_>>();
    let expected = expected.into_iter().cloned().collect::<HashSet<_>>();
    assert_eq!(actual, expected);
}

fn limit_order_json(
    symbol: &str,
    qty: &str,
    side: &str,
    limit_price: &str,
    client_order_id: &str,
) -> serde_json::Value {
    json!({
        "symbol": symbol,
        "qty": qty,
        "side": side,
        "type": "limit",
        "time_in_force": "gtc",
        "limit_price": limit_price,
        "extended_hours": true,
        "client_order_id": client_order_id,
        "order_class": "simple"
    })
}
