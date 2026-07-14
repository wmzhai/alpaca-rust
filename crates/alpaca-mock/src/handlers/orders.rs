use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, de::Error as _};

use alpaca_trade::orders::{
    AdvancedInstructions, CancelAllOrderResult, CreateRequest, OptionLegRequest, Order,
    OrderAssetClass, OrderClass, OrderSide, OrderType, PositionIntent, QueryOrderStatus,
    SortDirection, StopLoss, TakeProfit, TimeInForce,
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
            MockStateError::Forbidden(message) => Self::with_status(StatusCode::FORBIDDEN, message),
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
pub(crate) struct GetOrderQuery {
    nested: Option<bool>,
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
    #[serde(default, deserialize_with = "deserialize_asset_classes")]
    asset_class: Option<Vec<OrderAssetClass>>,
    before_order_id: Option<String>,
    after_order_id: Option<String>,
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
    advanced_instructions: Option<AdvancedInstructions>,
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
    advanced_instructions: Option<AdvancedInstructions>,
}

pub(crate) async fn orders_create(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Json(body): Json<CreateOrderBody>,
) -> RouteResult<Json<Order>> {
    CreateRequest {
        symbol: body.symbol.clone(),
        qty: body.qty,
        notional: body.notional,
        side: body.side,
        r#type: body.r#type,
        time_in_force: body.time_in_force,
        limit_price: body.limit_price,
        stop_price: body.stop_price,
        trail_price: body.trail_price,
        trail_percent: body.trail_percent,
        extended_hours: body.extended_hours,
        client_order_id: body.client_order_id.clone(),
        order_class: body.order_class,
        take_profit: body.take_profit.clone(),
        stop_loss: body.stop_loss.clone(),
        legs: body.legs.clone(),
        position_intent: body.position_intent,
        advanced_instructions: body.advanced_instructions.clone(),
    }
    .validate()
    .map_err(|error| MockHttpError::conflict(error.to_string()))?;

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
                advanced_instructions: body.advanced_instructions,
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
    validate_list_orders_query(&query)?;
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
            status: query.status,
            limit: query.limit,
            after: query.after,
            until: query.until,
            direction: query.direction,
            symbols,
            side: query.side,
            asset_classes: query.asset_class,
            nested: query.nested,
            before_order_id: query.before_order_id,
            after_order_id: query.after_order_id,
        },
    )))
}

fn validate_list_orders_query(query: &ListOrdersQuery) -> RouteResult<()> {
    if query.limit.is_some_and(|limit| !(1..=500).contains(&limit)) {
        return Err(MockHttpError::bad_request(
            "limit must be between 1 and 500".to_owned(),
        ));
    }
    if query.before_order_id.is_some() && query.after_order_id.is_some() {
        return Err(MockHttpError::bad_request(
            "before_order_id and after_order_id are mutually exclusive".to_owned(),
        ));
    }
    if (query.before_order_id.is_some() || query.after_order_id.is_some())
        && (query.after.is_some() || query.until.is_some())
    {
        return Err(MockHttpError::bad_request(
            "order ID cursors cannot be combined with after or until".to_owned(),
        ));
    }
    for (name, value) in [
        ("after", query.after.as_deref()),
        ("until", query.until.as_deref()),
        ("before_order_id", query.before_order_id.as_deref()),
        ("after_order_id", query.after_order_id.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            return Err(MockHttpError::bad_request(format!(
                "{name} must not be empty"
            )));
        }
    }
    Ok(())
}

fn deserialize_asset_classes<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<OrderAssetClass>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| {
            value
                .split(',')
                .filter(|item| !item.trim().is_empty())
                .map(|item| match item.trim() {
                    "us_equity" => Ok(OrderAssetClass::UsEquity),
                    "us_option" => Ok(OrderAssetClass::UsOption),
                    "crypto" => Ok(OrderAssetClass::Crypto),
                    "crypto_perp" => Ok(OrderAssetClass::CryptoPerp),
                    "treasury" => Ok(OrderAssetClass::Treasury),
                    "corporate" => Ok(OrderAssetClass::Corporate),
                    "global_equity" => Ok(OrderAssetClass::GlobalEquity),
                    "us_index" => Ok(OrderAssetClass::UsIndex),
                    "us_equity_chain" => Ok(OrderAssetClass::UsEquityChain),
                    "ipo" => Ok(OrderAssetClass::Ipo),
                    "all" => Ok(OrderAssetClass::All),
                    value => Err(D::Error::custom(format!(
                        "unsupported asset_class value {value}"
                    ))),
                })
                .collect::<Result<Vec<_>, D::Error>>()
        })
        .transpose()
}

pub(crate) async fn orders_get(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(order_id): Path<String>,
    Query(query): Query<GetOrderQuery>,
) -> RouteResult<Json<Order>> {
    let order = state
        .get_order(&account.api_key, &order_id, query.nested.unwrap_or(false))
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
    alpaca_trade::orders::ReplaceRequest {
        qty: body.qty,
        time_in_force: body.time_in_force,
        limit_price: body.limit_price,
        stop_price: body.stop_price,
        trail: body.trail,
        client_order_id: body.client_order_id.clone(),
        advanced_instructions: body.advanced_instructions.clone(),
    }
    .validate()
    .map_err(|error| MockHttpError::conflict(error.to_string()))?;

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
                advanced_instructions: body.advanced_instructions,
            },
        )
        .await?;
    Ok(Json(order))
}

pub(crate) async fn orders_cancel(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(order_id): Path<String>,
) -> RouteResult<StatusCode> {
    state.cancel_order(&account.api_key, &order_id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn orders_cancel_all(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
) -> RouteResult<(StatusCode, Json<Vec<CancelAllOrderResult>>)> {
    Ok((
        StatusCode::MULTI_STATUS,
        Json(state.cancel_all_orders(&account.api_key)),
    ))
}
