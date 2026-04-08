use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

mod account;
mod activities;
mod executions;
mod market_data;
mod positions;

use chrono::{SecondsFormat, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

use alpaca_trade::orders::{
    CancelAllOrderResult, Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
    StopLoss, TakeProfit, TimeInForce,
};
use alpaca_trade::positions::{
    ClosePositionBody, ClosePositionResult, ExercisePositionBody, Position,
};

use activities::{ActivityEvent, ActivityEventKind};
use executions::ExecutionFact;
pub use market_data::{DEFAULT_STOCK_SYMBOL, InstrumentSnapshot, LiveMarketDataBridge};
use positions::{OptionContractType, PositionBook, parse_option_symbol, project_position};

#[derive(Debug, Clone)]
pub struct MockServerState {
    inner: Arc<SharedState>,
}

#[derive(Debug)]
struct SharedState {
    accounts: RwLock<HashMap<String, VirtualAccountState>>,
    http_fault: RwLock<Option<InjectedHttpFault>>,
    market_data_bridge: Option<LiveMarketDataBridge>,
}

#[derive(Debug, Clone)]
pub(crate) struct VirtualAccountState {
    account_profile: account::AccountProfile,
    cash_ledger: account::CashLedger,
    orders: HashMap<String, StoredOrder>,
    client_order_ids: HashMap<String, String>,
    executions: Vec<ExecutionFact>,
    positions: PositionBook,
    activities: Vec<ActivityEvent>,
    sequence_clock: u64,
}

#[derive(Debug, Clone)]
struct StoredOrder {
    order: Order,
    request_side: OrderSide,
}

#[derive(Debug, Clone, Default)]
pub struct CreateOrderInput {
    pub symbol: Option<String>,
    pub qty: Option<Decimal>,
    pub notional: Option<Decimal>,
    pub side: Option<OrderSide>,
    pub order_type: Option<OrderType>,
    pub time_in_force: Option<TimeInForce>,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trail_price: Option<Decimal>,
    pub trail_percent: Option<Decimal>,
    pub extended_hours: Option<bool>,
    pub client_order_id: Option<String>,
    pub order_class: Option<OrderClass>,
    pub position_intent: Option<PositionIntent>,
    pub take_profit: Option<TakeProfit>,
    pub stop_loss: Option<StopLoss>,
}

#[derive(Debug, Clone, Default)]
pub struct ReplaceOrderInput {
    pub qty: Option<Decimal>,
    pub time_in_force: Option<TimeInForce>,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trail: Option<Decimal>,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClosePositionInput {
    pub qty: Option<Decimal>,
    pub percentage: Option<Decimal>,
}

#[derive(Debug, Clone, Default)]
pub struct ListOrdersFilter {
    pub status: Option<String>,
    pub symbols: Option<Vec<String>>,
    pub side: Option<OrderSide>,
    pub asset_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InjectedHttpFault {
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminStateResponse {
    pub account_count: usize,
    pub market_data_bridge_configured: bool,
    pub http_fault: Option<InjectedHttpFault>,
}

#[derive(Debug, Error)]
pub enum MarketDataBridgeError {
    #[error(transparent)]
    Data(#[from] alpaca_data::Error),
    #[error("market data unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MockStateError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    MarketDataUnavailable(String),
}

impl MockServerState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SharedState {
                accounts: RwLock::new(HashMap::new()),
                http_fault: RwLock::new(None),
                market_data_bridge: None,
            }),
        }
    }

    pub fn from_env() -> Result<Self, MarketDataBridgeError> {
        Ok(match LiveMarketDataBridge::from_env_optional()? {
            Some(bridge) => Self::new().with_market_data_bridge(bridge),
            None => Self::new(),
        })
    }

    #[must_use]
    pub fn with_market_data_bridge(mut self, bridge: LiveMarketDataBridge) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("mock state should be uniquely owned during configuration")
            .market_data_bridge = Some(bridge);
        self
    }

    #[must_use]
    pub fn market_data_bridge(&self) -> Option<&LiveMarketDataBridge> {
        self.inner.market_data_bridge.as_ref()
    }

    pub fn ensure_account(&self, api_key: &str) {
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
    }

    #[must_use]
    pub fn project_account(&self, api_key: &str) -> alpaca_trade::account::Account {
        self.ensure_account(api_key);
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        let state = accounts
            .get(api_key)
            .expect("account should exist after ensure_account");
        account::project_account(state)
    }

    pub async fn create_order(
        &self,
        api_key: &str,
        input: CreateOrderInput,
    ) -> Result<Order, MockStateError> {
        let order_class = input.order_class.clone().unwrap_or(OrderClass::Simple);
        if order_class == OrderClass::Mleg {
            return Err(MockStateError::Conflict(
                "alpaca-mock does not yet support mleg order simulation".to_owned(),
            ));
        }

        let request_side = input.side.clone().unwrap_or(OrderSide::Buy);
        if request_side == OrderSide::Unspecified {
            return Err(MockStateError::Conflict(
                "mock orders require an explicit buy or sell side".to_owned(),
            ));
        }

        let order_type = input.order_type.clone().unwrap_or(OrderType::Market);
        let time_in_force = input.time_in_force.clone().unwrap_or(TimeInForce::Day);
        let symbol = input
            .symbol
            .clone()
            .unwrap_or_else(|| DEFAULT_STOCK_SYMBOL.to_owned());
        let snapshot = self.instrument_snapshot(&symbol).await?;
        let reference_price = reference_price(&request_side, &snapshot);
        let qty = normalize_qty(input.qty, input.notional, reference_price)?;
        let fill_price =
            marketable_fill_price(&order_type, &request_side, input.limit_price, &snapshot);
        let client_order_id = input
            .client_order_id
            .unwrap_or_else(|| format!("mock-client-order-{}", now_millis()));

        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        if account.client_order_ids.contains_key(&client_order_id) {
            return Err(MockStateError::Conflict(format!(
                "client_order_id {client_order_id} already exists"
            )));
        }

        let order_id = account.next_order_id();
        let now = now_string();
        let order = Order {
            id: order_id.clone(),
            client_order_id: client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            filled_at: fill_price.map(|_| now.clone()),
            expired_at: None,
            expires_at: expires_at_for(&time_in_force),
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: None,
            asset_id: mock_asset_id(&symbol),
            symbol: symbol.clone(),
            asset_class: snapshot.asset_class.clone(),
            notional: input.notional,
            qty: Some(qty),
            filled_qty: fill_price.map_or(Decimal::ZERO, |_| qty),
            filled_avg_price: fill_price,
            order_class,
            order_type: order_type.clone(),
            r#type: order_type,
            side: request_side.clone(),
            position_intent: input.position_intent.clone(),
            time_in_force,
            limit_price: input.limit_price,
            stop_price: input.stop_price,
            status: if fill_price.is_some() {
                OrderStatus::Filled
            } else {
                OrderStatus::New
            },
            extended_hours: input.extended_hours.unwrap_or(false),
            legs: None,
            trail_percent: input.trail_percent,
            trail_price: input.trail_price,
            hwm: None,
            ratio_qty: None,
            take_profit: input.take_profit,
            stop_loss: input.stop_loss,
            subtag: None,
            source: None,
        };

        account
            .client_order_ids
            .insert(client_order_id, order_id.clone());
        account.orders.insert(
            order_id,
            StoredOrder {
                order: order.clone(),
                request_side: request_side.clone(),
            },
        );
        record_create_effects(account, &order, &request_side);

        Ok(order)
    }

    #[must_use]
    pub fn list_orders(&self, api_key: &str, filter: ListOrdersFilter) -> Vec<Order> {
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        let Some(account) = accounts.get(api_key) else {
            return Vec::new();
        };

        let symbol_filter = filter.symbols.map(|symbols| {
            symbols
                .into_iter()
                .map(|symbol| symbol.trim().to_owned())
                .filter(|symbol| !symbol.is_empty())
                .collect::<HashSet<_>>()
        });

        let mut orders = account
            .orders
            .values()
            .filter(|stored| {
                let order = &stored.order;
                matches_status_filter(order, filter.status.as_deref())
                    && symbol_filter
                        .as_ref()
                        .is_none_or(|symbols| symbols.contains(&order.symbol))
                    && filter.side.as_ref().is_none_or(|side| &order.side == side)
                    && filter
                        .asset_class
                        .as_deref()
                        .is_none_or(|asset_class| order.asset_class == asset_class)
            })
            .map(|stored| stored.order.clone())
            .collect::<Vec<_>>();
        orders.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        orders
    }

    #[must_use]
    pub fn get_order(&self, api_key: &str, order_id: &str) -> Option<Order> {
        self.inner
            .accounts
            .read()
            .expect("accounts lock should not poison")
            .get(api_key)
            .and_then(|account| account.orders.get(order_id))
            .map(|stored| stored.order.clone())
    }

    #[must_use]
    pub fn get_by_client_order_id(&self, api_key: &str, client_order_id: &str) -> Option<Order> {
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        let account = accounts.get(api_key)?;
        let order_id = account.client_order_ids.get(client_order_id)?;
        account
            .orders
            .get(order_id)
            .map(|stored| stored.order.clone())
    }

    pub async fn replace_order(
        &self,
        api_key: &str,
        order_id: &str,
        input: ReplaceOrderInput,
    ) -> Result<Order, MockStateError> {
        let current = {
            let accounts = self
                .inner
                .accounts
                .read()
                .expect("accounts lock should not poison");
            let account = accounts.get(api_key).ok_or_else(|| {
                MockStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            account.orders.get(order_id).cloned().ok_or_else(|| {
                MockStateError::NotFound(format!("order {order_id} was not found"))
            })?
        };

        if is_terminal_status(&current.order.status) {
            return Err(MockStateError::Conflict(format!(
                "order {order_id} is no longer replaceable"
            )));
        }

        let symbol = current.order.symbol.clone();
        let snapshot = self.instrument_snapshot(&symbol).await?;
        let request_side = current.request_side.clone();
        let replacement_limit_price = input.limit_price.or(current.order.limit_price);
        let replacement_qty = input.qty.or(current.order.qty);
        let replacement_client_order_id = input
            .client_order_id
            .clone()
            .unwrap_or_else(|| current.order.client_order_id.clone());
        let replacement_time_in_force = input
            .time_in_force
            .clone()
            .unwrap_or_else(|| current.order.time_in_force.clone());
        let fill_price = marketable_fill_price(
            &current.order.r#type,
            &request_side,
            replacement_limit_price,
            &snapshot,
        );
        let qty = normalize_qty(
            replacement_qty,
            current.order.notional,
            reference_price(&request_side, &snapshot),
        )?;

        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));

        if replacement_client_order_id != current.order.client_order_id
            && account
                .client_order_ids
                .contains_key(&replacement_client_order_id)
        {
            return Err(MockStateError::Conflict(format!(
                "client_order_id {replacement_client_order_id} already exists"
            )));
        }

        let now = now_string();
        let replacement_order_id = account.next_order_id();
        let replacement = Order {
            id: replacement_order_id.clone(),
            client_order_id: replacement_client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            filled_at: fill_price.map(|_| now.clone()),
            expired_at: None,
            expires_at: expires_at_for(&replacement_time_in_force),
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: Some(current.order.id.clone()),
            asset_id: current.order.asset_id.clone(),
            symbol: current.order.symbol.clone(),
            asset_class: current.order.asset_class.clone(),
            notional: current.order.notional,
            qty: Some(qty),
            filled_qty: fill_price.map_or(Decimal::ZERO, |_| qty),
            filled_avg_price: fill_price,
            order_class: current.order.order_class.clone(),
            order_type: current.order.order_type.clone(),
            r#type: current.order.r#type.clone(),
            side: request_side.clone(),
            position_intent: current.order.position_intent.clone(),
            time_in_force: replacement_time_in_force,
            limit_price: replacement_limit_price,
            stop_price: input.stop_price.or(current.order.stop_price),
            status: if fill_price.is_some() {
                OrderStatus::Filled
            } else {
                OrderStatus::New
            },
            extended_hours: current.order.extended_hours,
            legs: current.order.legs.clone(),
            trail_percent: current.order.trail_percent,
            trail_price: input.trail.or(current.order.trail_price),
            hwm: current.order.hwm,
            ratio_qty: current.order.ratio_qty,
            take_profit: current.order.take_profit.clone(),
            stop_loss: current.order.stop_loss.clone(),
            subtag: current.order.subtag.clone(),
            source: current.order.source.clone(),
        };

        let (current_order_id, current_client_order_id, current_symbol, current_asset_class) = {
            let current = account.orders.get_mut(order_id).ok_or_else(|| {
                MockStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            if is_terminal_status(&current.order.status) {
                return Err(MockStateError::Conflict(format!(
                    "order {order_id} is no longer replaceable"
                )));
            }
            current.order.status = OrderStatus::Replaced;
            current.order.updated_at = now.clone();
            current.order.replaced_at = Some(now.clone());
            current.order.replaced_by = Some(replacement.id.clone());
            (
                current.order.id.clone(),
                current.order.client_order_id.clone(),
                current.order.symbol.clone(),
                current.order.asset_class.clone(),
            )
        };

        if replacement_client_order_id == current.order.client_order_id {
            account
                .client_order_ids
                .insert(replacement_client_order_id.clone(), replacement.id.clone());
        } else {
            account
                .client_order_ids
                .insert(replacement_client_order_id.clone(), replacement.id.clone());
        }
        account.orders.insert(
            replacement.id.clone(),
            StoredOrder {
                order: replacement.clone(),
                request_side: request_side.clone(),
            },
        );

        let replaced_event = ActivityEvent::new(
            account.next_sequence(),
            ActivityEventKind::Replaced,
            current_order_id,
            current_client_order_id,
            Some(replacement.id.clone()),
            Some(OrderStatus::Replaced),
            current_symbol,
            current_asset_class,
            now.clone(),
            Decimal::ZERO,
        );
        account.activities.push(replaced_event);
        record_post_replace_effects(account, &replacement, &request_side);

        Ok(replacement)
    }

    pub fn cancel_order(&self, api_key: &str, order_id: &str) -> Result<(), MockStateError> {
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let now = now_string();
        let (order_id, client_order_id, symbol, asset_class) = {
            let stored = account.orders.get_mut(order_id).ok_or_else(|| {
                MockStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            if is_terminal_status(&stored.order.status) {
                return Err(MockStateError::Conflict(format!(
                    "order {order_id} is no longer cancelable"
                )));
            }
            stored.order.status = OrderStatus::Canceled;
            stored.order.updated_at = now.clone();
            stored.order.canceled_at = Some(now.clone());
            (
                stored.order.id.clone(),
                stored.order.client_order_id.clone(),
                stored.order.symbol.clone(),
                stored.order.asset_class.clone(),
            )
        };
        let sequence = account.next_sequence();
        account.activities.push(ActivityEvent::new(
            sequence,
            ActivityEventKind::Canceled,
            order_id,
            client_order_id,
            None,
            Some(OrderStatus::Canceled),
            symbol,
            asset_class,
            now,
            Decimal::ZERO,
        ));
        Ok(())
    }

    pub fn cancel_all_orders(&self, api_key: &str) -> Vec<CancelAllOrderResult> {
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let cancelable_ids = account
            .orders
            .iter()
            .filter_map(|(order_id, stored)| {
                if is_terminal_status(&stored.order.status) {
                    None
                } else {
                    Some(order_id.clone())
                }
            })
            .collect::<Vec<_>>();

        let mut results = Vec::with_capacity(cancelable_ids.len());
        for order_id in cancelable_ids {
            let now = now_string();
            let body = {
                let stored = account
                    .orders
                    .get_mut(&order_id)
                    .expect("cancelable order should remain present");
                stored.order.status = OrderStatus::Canceled;
                stored.order.updated_at = now.clone();
                stored.order.canceled_at = Some(now.clone());
                stored.order.clone()
            };
            let sequence = account.next_sequence();
            account.activities.push(ActivityEvent::new(
                sequence,
                ActivityEventKind::Canceled,
                body.id.clone(),
                body.client_order_id.clone(),
                None,
                Some(OrderStatus::Canceled),
                body.symbol.clone(),
                body.asset_class.clone(),
                now,
                Decimal::ZERO,
            ));
            results.push(CancelAllOrderResult {
                id: body.id.clone(),
                status: 200,
                body: Some(body),
            });
        }

        results
    }

    pub async fn list_positions(&self, api_key: &str) -> Result<Vec<Position>, MockStateError> {
        let open_positions = {
            let accounts = self
                .inner
                .accounts
                .read()
                .expect("accounts lock should not poison");
            accounts
                .get(api_key)
                .map(|account| account.positions.list_open_positions())
                .unwrap_or_default()
        };

        let mut projected = Vec::with_capacity(open_positions.len());
        for position in open_positions {
            let snapshot = self
                .instrument_snapshot(&position.instrument_identity.symbol)
                .await?;
            projected.push(public_position_from_projection(project_position(
                &position, &snapshot,
            )));
        }
        projected.sort_by(|left, right| left.symbol.cmp(&right.symbol));

        Ok(projected)
    }

    pub async fn get_position(
        &self,
        api_key: &str,
        symbol_or_asset_id: &str,
    ) -> Result<Position, MockStateError> {
        let position = {
            let accounts = self
                .inner
                .accounts
                .read()
                .expect("accounts lock should not poison");
            let account = accounts.get(api_key).ok_or_else(|| {
                MockStateError::NotFound(format!("position {symbol_or_asset_id} was not found"))
            })?;
            account
                .positions
                .find_open_position(symbol_or_asset_id)
                .ok_or_else(|| {
                    MockStateError::NotFound(format!("position {symbol_or_asset_id} was not found"))
                })?
        };
        let snapshot = self
            .instrument_snapshot(&position.instrument_identity.symbol)
            .await?;

        Ok(public_position_from_projection(project_position(
            &position, &snapshot,
        )))
    }

    pub async fn close_position(
        &self,
        api_key: &str,
        symbol_or_asset_id: &str,
        input: ClosePositionInput,
    ) -> Result<ClosePositionBody, MockStateError> {
        let position = {
            let accounts = self
                .inner
                .accounts
                .read()
                .expect("accounts lock should not poison");
            let account = accounts.get(api_key).ok_or_else(|| {
                MockStateError::NotFound(format!("position {symbol_or_asset_id} was not found"))
            })?;
            account
                .positions
                .find_open_position(symbol_or_asset_id)
                .ok_or_else(|| {
                    MockStateError::NotFound(format!("position {symbol_or_asset_id} was not found"))
                })?
        };
        let snapshot = self
            .instrument_snapshot(&position.instrument_identity.symbol)
            .await?;
        let close_qty = resolve_close_qty(&position, &input)?;
        let request_side = if position.net_qty > Decimal::ZERO {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };
        let price = reference_price(&request_side, &snapshot);
        let now = now_string();

        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let order = Order {
            id: account.next_order_id(),
            client_order_id: format!("mock-position-close-{}", now_millis()),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            filled_at: Some(now.clone()),
            expired_at: None,
            expires_at: None,
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: None,
            asset_id: position.instrument_identity.asset_id.clone(),
            symbol: position.instrument_identity.symbol.clone(),
            asset_class: position.instrument_identity.asset_class.clone(),
            notional: None,
            qty: Some(close_qty),
            filled_qty: close_qty,
            filled_avg_price: Some(price),
            order_class: OrderClass::Simple,
            order_type: OrderType::Market,
            r#type: OrderType::Market,
            side: request_side.clone(),
            position_intent: closing_position_intent(&position, &request_side),
            time_in_force: TimeInForce::Day,
            limit_price: None,
            stop_price: None,
            status: OrderStatus::Filled,
            extended_hours: false,
            legs: None,
            trail_percent: None,
            trail_price: None,
            hwm: None,
            ratio_qty: None,
            take_profit: None,
            stop_loss: None,
            subtag: None,
            source: None,
        };
        account
            .client_order_ids
            .insert(order.client_order_id.clone(), order.id.clone());
        account.orders.insert(
            order.id.clone(),
            StoredOrder {
                order: order.clone(),
                request_side: request_side.clone(),
            },
        );
        apply_fill_effects(account, &order, &request_side);

        Ok(ClosePositionBody::from(order))
    }

    pub async fn close_all_positions(
        &self,
        api_key: &str,
        cancel_orders: bool,
    ) -> Result<Vec<ClosePositionResult>, MockStateError> {
        if cancel_orders {
            let _ = self.cancel_all_orders(api_key);
        }

        let positions = {
            let accounts = self
                .inner
                .accounts
                .read()
                .expect("accounts lock should not poison");
            accounts
                .get(api_key)
                .map(|account| account.positions.list_open_positions())
                .unwrap_or_default()
        };

        let mut results = Vec::with_capacity(positions.len());
        for position in positions {
            let symbol = position.instrument_identity.symbol.clone();
            let body = self
                .close_position(api_key, &symbol, ClosePositionInput::default())
                .await?;
            results.push(ClosePositionResult {
                symbol,
                status: 200,
                body: Some(body),
            });
        }

        Ok(results)
    }

    pub fn do_not_exercise_position(
        &self,
        api_key: &str,
        symbol_or_contract_id: &str,
    ) -> Result<(), MockStateError> {
        let now = now_string();
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let position = account
            .positions
            .find_open_position(symbol_or_contract_id)
            .ok_or_else(|| {
                MockStateError::NotFound(format!("position {symbol_or_contract_id} was not found"))
            })?;
        ensure_exercisable_long_option_position(&position)?;
        account
            .positions
            .record_do_not_exercise(&position.instrument_identity.symbol, &now);
        let sequence = account.next_sequence();
        let action_id = format!("mock-dne-{}", now_millis());
        account.activities.push(ActivityEvent::new(
            sequence,
            ActivityEventKind::DoNotExercise,
            action_id.clone(),
            action_id,
            None,
            None,
            position.instrument_identity.symbol,
            position.instrument_identity.asset_class,
            now,
            Decimal::ZERO,
        ));
        Ok(())
    }

    pub fn exercise_position(
        &self,
        api_key: &str,
        symbol_or_contract_id: &str,
    ) -> Result<ExercisePositionBody, MockStateError> {
        let now = now_string();
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let position = account
            .positions
            .find_open_position(symbol_or_contract_id)
            .ok_or_else(|| {
                MockStateError::NotFound(format!("position {symbol_or_contract_id} was not found"))
            })?;
        ensure_exercisable_long_option_position(&position)?;
        let parsed =
            parse_option_symbol(&position.instrument_identity.symbol).ok_or_else(|| {
                MockStateError::Conflict(format!(
                    "option symbol {} is not a parseable OCC contract",
                    position.instrument_identity.symbol
                ))
            })?;
        let option_qty = position.net_qty.abs();
        let share_qty = option_qty * Decimal::new(100, 0);
        let option_execution = ExecutionFact::new(
            account.next_sequence(),
            format!("mock-exercise-option-{}", now_millis()),
            position.instrument_identity.asset_id.clone(),
            position.instrument_identity.symbol.clone(),
            position.instrument_identity.asset_class.clone(),
            OrderSide::Sell,
            Some(PositionIntent::SellToClose),
            option_qty,
            Decimal::ZERO,
            now.clone(),
        );
        let (underlying_side, position_intent) = match parsed.contract_type {
            OptionContractType::Call => (OrderSide::Buy, Some(PositionIntent::BuyToOpen)),
            OptionContractType::Put => (OrderSide::Sell, Some(PositionIntent::SellToOpen)),
        };
        let underlying_execution = ExecutionFact::new(
            account.next_sequence(),
            format!("mock-exercise-underlying-{}", now_millis()),
            mock_asset_id(&parsed.underlying_symbol),
            parsed.underlying_symbol.clone(),
            "us_equity".to_owned(),
            underlying_side.clone(),
            position_intent,
            share_qty,
            parsed.strike_price,
            now.clone(),
        );
        account
            .positions
            .clear_do_not_exercise_override(&position.instrument_identity.symbol);
        account.positions.apply_execution(&option_execution);
        account.executions.push(option_execution);
        let underlying_cash_delta = signed_cash_delta(
            &underlying_side,
            underlying_execution.qty,
            underlying_execution.price,
        );
        account.cash_ledger.apply_delta(underlying_cash_delta);
        account.positions.apply_execution(&underlying_execution);
        account.executions.push(underlying_execution);
        let sequence = account.next_sequence();
        let action_id = format!("mock-exercise-{}", now_millis());
        account.activities.push(ActivityEvent::new(
            sequence,
            ActivityEventKind::Exercised,
            action_id.clone(),
            action_id,
            None,
            None,
            position.instrument_identity.symbol,
            position.instrument_identity.asset_class,
            now,
            underlying_cash_delta,
        ));

        Ok(ExercisePositionBody {
            qty_exercised: option_qty,
            qty_remaining: Decimal::ZERO,
        })
    }

    pub fn reset(&self) {
        self.inner
            .accounts
            .write()
            .expect("accounts lock should not poison")
            .clear();
        self.clear_http_fault();
    }

    pub fn set_http_fault(&self, fault: InjectedHttpFault) {
        *self
            .inner
            .http_fault
            .write()
            .expect("fault lock should not poison") = Some(fault);
    }

    pub fn clear_http_fault(&self) {
        *self
            .inner
            .http_fault
            .write()
            .expect("fault lock should not poison") = None;
    }

    #[must_use]
    pub fn http_fault(&self) -> Option<InjectedHttpFault> {
        self.inner
            .http_fault
            .read()
            .expect("fault lock should not poison")
            .clone()
    }

    #[must_use]
    pub fn account_count(&self) -> usize {
        self.inner
            .accounts
            .read()
            .expect("accounts lock should not poison")
            .len()
    }

    #[must_use]
    pub fn admin_state(&self) -> AdminStateResponse {
        AdminStateResponse {
            account_count: self.account_count(),
            market_data_bridge_configured: self.market_data_bridge().is_some(),
            http_fault: self.http_fault(),
        }
    }

    async fn instrument_snapshot(
        &self,
        symbol: &str,
    ) -> Result<InstrumentSnapshot, MockStateError> {
        let bridge = self.market_data_bridge().cloned().ok_or_else(|| {
            MockStateError::MarketDataUnavailable(
                "mock order simulation requires ALPACA_DATA_* credentials and a configured market data bridge".to_owned(),
            )
        })?;
        bridge
            .instrument_snapshot(symbol)
            .await
            .map_err(|error| MockStateError::MarketDataUnavailable(error.to_string()))
    }
}

impl Default for MockServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualAccountState {
    fn new(api_key: &str) -> Self {
        Self {
            account_profile: account::AccountProfile::new(api_key),
            cash_ledger: account::CashLedger::seeded_default(),
            orders: HashMap::new(),
            client_order_ids: HashMap::new(),
            executions: Vec::new(),
            positions: PositionBook::default(),
            activities: Vec::new(),
            sequence_clock: 0,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence_clock += 1;
        self.sequence_clock
    }

    fn next_order_id(&mut self) -> String {
        format!("mock-order-{}-{}", now_millis(), self.next_sequence())
    }
}

impl InjectedHttpFault {
    pub fn new(status: u16, message: impl Into<String>) -> Result<Self, String> {
        if !(100..=599).contains(&status) {
            return Err(format!(
                "status must be a valid HTTP status code, got {status}"
            ));
        }

        let message = message.into();
        if message.trim().is_empty() {
            return Err("message must not be empty".to_owned());
        }

        Ok(Self { status, message })
    }

    pub fn status_code(&self) -> Result<axum::http::StatusCode, String> {
        axum::http::StatusCode::from_u16(self.status)
            .map_err(|error| format!("invalid fault status {}: {error}", self.status))
    }
}

pub(crate) fn cash_balance(state: &VirtualAccountState) -> Decimal {
    state.cash_ledger.cash_balance()
}

pub(crate) fn account_profile(state: &VirtualAccountState) -> &account::AccountProfile {
    &state.account_profile
}

fn normalize_qty(
    qty: Option<Decimal>,
    notional: Option<Decimal>,
    price: Decimal,
) -> Result<Decimal, MockStateError> {
    let qty = match qty {
        Some(qty) => qty,
        None => match notional {
            Some(notional) => (notional / price).round_dp(8),
            None => Decimal::ONE,
        },
    };

    if qty <= Decimal::ZERO {
        return Err(MockStateError::Conflict(
            "order quantity must be greater than 0".to_owned(),
        ));
    }

    Ok(qty)
}

fn resolve_close_qty(
    position: &positions::InstrumentPosition,
    input: &ClosePositionInput,
) -> Result<Decimal, MockStateError> {
    let available = position.net_qty.abs();
    let qty = if let Some(qty) = input.qty {
        qty
    } else if let Some(percentage) = input.percentage {
        if percentage <= Decimal::ZERO || percentage > Decimal::new(100, 0) {
            return Err(MockStateError::Conflict(
                "close percentage must be greater than 0 and at most 100".to_owned(),
            ));
        }
        (available * percentage / Decimal::new(100, 0)).round_dp(8)
    } else {
        available
    };

    if qty <= Decimal::ZERO {
        return Err(MockStateError::Conflict(
            "close quantity must be greater than 0".to_owned(),
        ));
    }
    if qty > available {
        return Err(MockStateError::Conflict(format!(
            "close quantity {qty} exceeds available position quantity {available}"
        )));
    }

    Ok(qty)
}

fn reference_price(side: &OrderSide, snapshot: &InstrumentSnapshot) -> Decimal {
    match side {
        OrderSide::Buy => snapshot.ask,
        OrderSide::Sell => snapshot.bid,
        OrderSide::Unspecified => snapshot.mid_price(),
    }
}

fn marketable_fill_price(
    order_type: &OrderType,
    side: &OrderSide,
    limit_price: Option<Decimal>,
    snapshot: &InstrumentSnapshot,
) -> Option<Decimal> {
    match order_type {
        OrderType::Market => Some(reference_price(side, snapshot)),
        OrderType::Limit => match side {
            OrderSide::Buy => limit_price
                .filter(|limit| *limit >= snapshot.ask)
                .map(|_| snapshot.ask),
            OrderSide::Sell => limit_price
                .filter(|limit| *limit <= snapshot.bid)
                .map(|_| snapshot.bid),
            OrderSide::Unspecified => None,
        },
        OrderType::Stop
        | OrderType::StopLimit
        | OrderType::TrailingStop
        | OrderType::Unspecified => None,
    }
}

fn record_create_effects(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
) {
    if order.status == OrderStatus::Filled {
        apply_fill_effects(account, order, request_side);
    } else {
        let sequence = account.next_sequence();
        account.activities.push(ActivityEvent::new(
            sequence,
            ActivityEventKind::New,
            order.id.clone(),
            order.client_order_id.clone(),
            None,
            Some(order.status.clone()),
            order.symbol.clone(),
            order.asset_class.clone(),
            order.created_at.clone(),
            Decimal::ZERO,
        ));
    }
}

fn record_post_replace_effects(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
) {
    if order.status == OrderStatus::Filled {
        apply_fill_effects(account, order, request_side);
    } else {
        let sequence = account.next_sequence();
        account.activities.push(ActivityEvent::new(
            sequence,
            ActivityEventKind::New,
            order.id.clone(),
            order.client_order_id.clone(),
            order.replaces.clone(),
            Some(order.status.clone()),
            order.symbol.clone(),
            order.asset_class.clone(),
            order.created_at.clone(),
            Decimal::ZERO,
        ));
    }
}

fn apply_fill_effects(account: &mut VirtualAccountState, order: &Order, request_side: &OrderSide) {
    let price = order
        .filled_avg_price
        .expect("filled mock order should always have filled_avg_price");
    let qty = order.filled_qty;
    let cash_delta = signed_cash_delta(request_side, qty, price);
    account.cash_ledger.apply_delta(cash_delta);
    let execution_sequence = account.next_sequence();
    let execution = ExecutionFact::new(
        execution_sequence,
        order.id.clone(),
        order.asset_id.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        request_side.clone(),
        order.position_intent.clone(),
        qty,
        price,
        order
            .filled_at
            .clone()
            .unwrap_or_else(|| order.updated_at.clone()),
    );
    account.positions.apply_execution(&execution);
    account.executions.push(execution);
    let activity_sequence = account.next_sequence();
    account.activities.push(ActivityEvent::new(
        activity_sequence,
        ActivityEventKind::Filled,
        order.id.clone(),
        order.client_order_id.clone(),
        order.replaces.clone(),
        Some(OrderStatus::Filled),
        order.symbol.clone(),
        order.asset_class.clone(),
        order
            .filled_at
            .clone()
            .unwrap_or_else(|| order.updated_at.clone()),
        cash_delta,
    ));
}

fn signed_cash_delta(side: &OrderSide, qty: Decimal, price: Decimal) -> Decimal {
    let gross = (price * qty).round_dp(8);
    match side {
        OrderSide::Buy => -gross,
        OrderSide::Sell => gross,
        OrderSide::Unspecified => Decimal::ZERO,
    }
}

fn is_terminal_status(status: &OrderStatus) -> bool {
    matches!(
        status,
        OrderStatus::Filled
            | OrderStatus::Canceled
            | OrderStatus::Expired
            | OrderStatus::Replaced
            | OrderStatus::Rejected
            | OrderStatus::Suspended
            | OrderStatus::DoneForDay
            | OrderStatus::Stopped
            | OrderStatus::Calculated
    )
}

fn matches_status_filter(order: &Order, status: Option<&str>) -> bool {
    match status {
        None => true,
        Some(value) if value.eq_ignore_ascii_case("all") => true,
        Some(value) if value.eq_ignore_ascii_case("open") => !is_terminal_status(&order.status),
        Some(value) if value.eq_ignore_ascii_case("closed") => is_terminal_status(&order.status),
        Some(_) => true,
    }
}

fn mock_asset_id(symbol: &str) -> String {
    let mut sanitized = String::with_capacity(symbol.len());
    for ch in symbol.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }
    format!("mock-asset-{}", sanitized.trim_matches('-'))
}

fn public_position_from_projection(projected: positions::ProjectedPosition) -> Position {
    Position {
        asset_id: projected.asset_id,
        symbol: projected.symbol,
        exchange: projected.exchange,
        asset_class: projected.asset_class,
        asset_marginable: projected.asset_marginable,
        side: projected.side,
        qty: projected.qty,
        avg_entry_price: projected.avg_entry_price,
        market_value: projected.market_value,
        cost_basis: projected.cost_basis,
        unrealized_pl: projected.unrealized_pl,
        unrealized_plpc: projected.unrealized_plpc,
        current_price: projected.current_price,
        lastday_price: projected.lastday_price,
        change_today: projected.change_today,
        qty_available: projected.qty_available,
    }
}

fn closing_position_intent(
    position: &positions::InstrumentPosition,
    request_side: &OrderSide,
) -> Option<PositionIntent> {
    if position.instrument_identity.asset_class != "us_option" {
        return None;
    }

    match request_side {
        OrderSide::Buy => Some(PositionIntent::BuyToClose),
        OrderSide::Sell => Some(PositionIntent::SellToClose),
        OrderSide::Unspecified => None,
    }
}

fn ensure_exercisable_long_option_position(
    position: &positions::InstrumentPosition,
) -> Result<(), MockStateError> {
    if position.instrument_identity.asset_class != "us_option" {
        return Err(MockStateError::Conflict(format!(
            "position {} is not an option contract",
            position.instrument_identity.symbol
        )));
    }
    if position.net_qty <= Decimal::ZERO {
        return Err(MockStateError::Conflict(format!(
            "position {} must be a long option position to use exercise controls",
            position.instrument_identity.symbol
        )));
    }

    Ok(())
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis()
}

fn expires_at_for(time_in_force: &TimeInForce) -> Option<String> {
    match time_in_force {
        TimeInForce::Gtd => Some(now_string()),
        TimeInForce::Day
        | TimeInForce::Gtc
        | TimeInForce::Opg
        | TimeInForce::Cls
        | TimeInForce::Ioc
        | TimeInForce::Fok => None,
    }
}
