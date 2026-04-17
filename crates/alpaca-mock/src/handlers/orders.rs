use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::Deserialize;

use alpaca_trade::orders::{
    CancelAllOrderResult, OptionLegRequest, Order, OrderClass, OrderSide, OrderType,
    PositionIntent, QueryOrderStatus, SortDirection, StopLoss, TakeProfit, TimeInForce,
};

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::{
    CreateOrderInput, ListOrdersFilter, MockServerState, MockStateError, ReplaceOrderInput,
};

type RouteResult<T> = Result<T, MockHttpError>;

impl From<MockStateError> for MockHttpError {
    fn from(error: MockStateError) -> Self {
        match error {
            MockStateError::NotFound(message) => Self::not_found(message),
            MockStateError::Conflict(message) => Self::conflict(message),
            MockStateError::MarketDataUnavailable(message) => Self::internal(message),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ByClientOrderIdQuery {
    client_order_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct ListOrdersQuery {
    status: Option<QueryOrderStatus>,
    limit: Option<u32>,
    after: Option<String>,
    until: Option<String>,
    direction: Option<SortDirection>,
    nested: Option<bool>,
    symbols: Option<String>,
    side: Option<OrderSide>,
    asset_class: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateOrderBody {
    symbol: Option<String>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    qty: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    notional: Option<Decimal>,
    side: Option<OrderSide>,
    #[serde(rename = "type")]
    r#type: Option<OrderType>,
    time_in_force: Option<TimeInForce>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    limit_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    stop_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    trail_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    trail_percent: Option<Decimal>,
    extended_hours: Option<bool>,
    client_order_id: Option<String>,
    order_class: Option<OrderClass>,
    take_profit: Option<TakeProfit>,
    stop_loss: Option<StopLoss>,
    legs: Option<Vec<OptionLegRequest>>,
    position_intent: Option<PositionIntent>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct ReplaceOrderBody {
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    qty: Option<Decimal>,
    time_in_force: Option<TimeInForce>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    limit_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    stop_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    trail: Option<Decimal>,
    client_order_id: Option<String>,
}

pub(crate) async fn orders_create(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Json(body): Json<CreateOrderBody>,
) -> RouteResult<Json<Order>> {
    let order = state
        .create_order(
            &account.api_key,
            CreateOrderInput {
                symbol: body.symbol,
                qty: body.qty,
                notional: body.notional,
                side: body.side,
                order_type: body.r#type,
                time_in_force: body.time_in_force,
                limit_price: body.limit_price,
                stop_price: body.stop_price,
                trail_price: body.trail_price,
                trail_percent: body.trail_percent,
                extended_hours: body.extended_hours,
                client_order_id: body.client_order_id,
                order_class: body.order_class,
                position_intent: body.position_intent,
                take_profit: body.take_profit,
                stop_loss: body.stop_loss,
                legs: body.legs,
            },
        )
        .await?;
    Ok(Json(order))
}

pub(crate) async fn orders_list(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<ListOrdersQuery>,
) -> RouteResult<Json<Vec<Order>>> {
    let _ = (
        query.limit,
        query.after,
        query.until,
        query.direction,
        query.nested,
    );
    let symbols = query.symbols.map(|symbols| {
        symbols
            .split(',')
            .map(|symbol| symbol.trim().to_owned())
            .filter(|symbol| !symbol.is_empty())
            .collect::<Vec<_>>()
    });

    Ok(Json(state.list_orders(
        &account.api_key,
        ListOrdersFilter {
            status: query.status.map(|status| status.to_string()),
            symbols,
            side: query.side,
            asset_class: query.asset_class,
            nested: query.nested,
        },
    )))
}

pub(crate) async fn orders_get(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(order_id): Path<String>,
) -> RouteResult<Json<Order>> {
    let order = state
        .get_order(&account.api_key, &order_id)
        .ok_or_else(|| MockHttpError::not_found(format!("order {order_id} was not found")))?;
    Ok(Json(order))
}

pub(crate) async fn orders_get_by_client_order_id(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<ByClientOrderIdQuery>,
) -> RouteResult<Json<Order>> {
    let order = state
        .get_by_client_order_id(&account.api_key, &query.client_order_id)
        .ok_or_else(|| {
            MockHttpError::not_found(format!(
                "client_order_id {} was not found",
                query.client_order_id
            ))
        })?;
    Ok(Json(order))
}

pub(crate) async fn orders_replace(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(order_id): Path<String>,
    Json(body): Json<ReplaceOrderBody>,
) -> RouteResult<Json<Order>> {
    let order = state
        .replace_order(
            &account.api_key,
            &order_id,
            ReplaceOrderInput {
                qty: body.qty,
                time_in_force: body.time_in_force,
                limit_price: body.limit_price,
                stop_price: body.stop_price,
                trail: body.trail,
                client_order_id: body.client_order_id,
            },
        )
        .await?;
    Ok(Json(order))
}

pub(crate) async fn orders_cancel(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(order_id): Path<String>,
) -> RouteResult<axum::http::StatusCode> {
    state.cancel_order(&account.api_key, &order_id)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub(crate) async fn orders_cancel_all(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
) -> RouteResult<Json<Vec<CancelAllOrderResult>>> {
    Ok(Json(state.cancel_all_orders(&account.api_key)))
}
