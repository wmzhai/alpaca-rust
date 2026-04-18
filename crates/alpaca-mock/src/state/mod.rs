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

use alpaca_trade::activities::Activity;
use alpaca_trade::orders::{
    CancelAllOrderResult, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    PositionIntent, SortDirection, StopLoss, TakeProfit, TimeInForce,
};
use alpaca_trade::positions::{
    ClosePositionBody, ClosePositionResult, ExercisePositionBody, Position,
};

use activities::{
    is_public_activity, matches_activity_type, project_activity, ActivityEvent, ActivityEventKind,
};
use executions::ExecutionFact;
pub use market_data::{InstrumentSnapshot, LiveMarketDataBridge, DEFAULT_STOCK_SYMBOL};
use positions::{parse_option_symbol, project_position, OptionContractType, PositionBook};

#[derive(Debug, Clone)]
pub struct MockServerState {
    inner: Arc<SharedState>,
}

#[derive(Debug)]
struct SharedState {
    accounts: RwLock<HashMap<String, VirtualAccountState>>,
    http_fault: RwLock<Option<InjectedHttpFault>>,
    market_data_bridge: Option<LiveMarketDataBridge>,
    market_data_cache: RwLock<HashMap<String, InstrumentSnapshot>>,
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
    pub legs: Option<Vec<OptionLegRequest>>,
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
    pub nested: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ListActivitiesFilter {
    pub activity_types: Option<Vec<String>>,
    pub date: Option<String>,
    pub until: Option<String>,
    pub after: Option<String>,
    pub direction: Option<SortDirection>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
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
                market_data_cache: RwLock::new(HashMap::new()),
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
        let request_side = input.side.clone().unwrap_or(OrderSide::Buy);
        if request_side == OrderSide::Unspecified {
            return Err(MockStateError::Conflict(
                "mock orders require an explicit buy or sell side".to_owned(),
            ));
        }

        let order_type = input.order_type.clone().unwrap_or(OrderType::Market);
        let time_in_force = input.time_in_force.clone().unwrap_or(TimeInForce::Day);
        let requested_symbol = input
            .symbol
            .clone()
            .unwrap_or_else(|| DEFAULT_STOCK_SYMBOL.to_owned());
        let requested_legs = input.legs.clone();
        let requested_position_intent = input.position_intent.clone();
        let requested_take_profit = input.take_profit.clone();
        let requested_stop_loss = input.stop_loss.clone();
        let client_order_id = input
            .client_order_id
            .unwrap_or_else(|| format!("mock-client-order-{}", now_millis()));
        let market_quotes = self
            .resolve_market_quotes(
                if order_class == OrderClass::Mleg {
                    None
                } else {
                    Some(requested_symbol.as_str())
                },
                input.legs.as_deref(),
            )
            .await?;
        let qty = if order_class == OrderClass::Mleg {
            normalize_qty(input.qty, None, Decimal::ONE)?
        } else {
            let snapshot = market_quotes.get(&requested_symbol).ok_or_else(|| {
                MockStateError::MarketDataUnavailable(format!(
                    "mock order creation for {requested_symbol} requires live market data"
                ))
            })?;
            let reference_price = reference_price(&request_side, snapshot);
            normalize_qty(input.qty, input.notional, reference_price)?
        };

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
        let order_time_in_force = time_in_force.clone();
        let order_type_for_legs = order_type.clone();
        let mut order = Order {
            id: order_id.clone(),
            client_order_id: client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            filled_at: None,
            expired_at: None,
            expires_at: expires_at_for(&time_in_force),
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: None,
            asset_id: if order_class == OrderClass::Mleg {
                String::new()
            } else {
                mock_asset_id(&requested_symbol)
            },
            symbol: if order_class == OrderClass::Mleg {
                String::new()
            } else {
                requested_symbol.clone()
            },
            asset_class: if order_class == OrderClass::Mleg {
                String::new()
            } else {
                market_quotes
                    .get(&requested_symbol)
                    .expect("simple order market quote should exist")
                    .asset_class
                    .clone()
            },
            notional: input.notional,
            qty: Some(qty),
            filled_qty: Decimal::ZERO,
            filled_avg_price: None,
            order_class: order_class.clone(),
            order_type: order_type.clone(),
            r#type: order_type,
            side: if order_class == OrderClass::Mleg {
                OrderSide::Unspecified
            } else {
                request_side.clone()
            },
            position_intent: if order_class == OrderClass::Mleg {
                None
            } else {
                requested_position_intent.clone()
            },
            time_in_force: order_time_in_force,
            limit_price: input.limit_price,
            stop_price: input.stop_price,
            status: OrderStatus::New,
            extended_hours: input.extended_hours.unwrap_or(false),
            legs: build_order_legs(
                &order_class,
                qty,
                order_type_for_legs,
                time_in_force.clone(),
                &now,
                &requested_symbol,
                &market_quotes,
                &request_side,
                requested_position_intent.clone(),
                requested_take_profit.clone(),
                requested_stop_loss.clone(),
                requested_legs.as_deref(),
                None,
            )?,
            trail_percent: input.trail_percent,
            trail_price: input.trail_price,
            hwm: None,
            ratio_qty: None,
            take_profit: requested_take_profit,
            stop_loss: requested_stop_loss,
            subtag: None,
            source: None,
        };
        if order_class == OrderClass::Mleg {
            apply_mleg_fill_rules(&mut order, &request_side, &market_quotes);
        } else {
            let snapshot = market_quotes
                .get(&requested_symbol)
                .expect("simple order market quote should exist");
            let fill_price = marketable_fill_price(
                &order.order_type,
                &request_side,
                order.limit_price,
                snapshot,
            );
            order.filled_at = fill_price.map(|_| now.clone());
            order.filled_qty = fill_price.map_or(Decimal::ZERO, |_| qty);
            order.filled_avg_price = fill_price;
            order.status = if fill_price.is_some() {
                OrderStatus::Filled
            } else {
                OrderStatus::New
            };
        }

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
            .map(|stored| order_for_list(&stored.order, filter.nested.unwrap_or(false)))
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

    #[must_use]
    pub fn list_activities(&self, api_key: &str, filter: ListActivitiesFilter) -> Vec<Activity> {
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        let Some(account) = accounts.get(api_key) else {
            return Vec::new();
        };

        let requested_types = filter.activity_types.map(|activity_types| {
            activity_types
                .into_iter()
                .map(|activity_type| activity_type.trim().to_owned())
                .filter(|activity_type| !activity_type.is_empty())
                .collect::<Vec<_>>()
        });
        let direction = filter.direction.unwrap_or(SortDirection::Desc);

        let mut events = account
            .activities
            .iter()
            .filter(|event| is_public_activity(event))
            .filter(|event| {
                requested_types.as_ref().is_none_or(|activity_types| {
                    activity_types
                        .iter()
                        .any(|activity_type| matches_activity_type(event, activity_type))
                })
            })
            .filter(|event| {
                filter
                    .date
                    .as_deref()
                    .is_none_or(|date| event_matches_date(event, date))
            })
            .filter(|event| {
                filter
                    .after
                    .as_deref()
                    .is_none_or(|after| event.occurred_at.as_str() >= after)
            })
            .filter(|event| {
                filter
                    .until
                    .as_deref()
                    .is_none_or(|until| event.occurred_at.as_str() <= until)
            })
            .cloned()
            .collect::<Vec<_>>();

        events.sort_by(|left, right| left.sequence.cmp(&right.sequence));
        if matches!(direction, SortDirection::Desc) {
            events.reverse();
        }

        let mut activities = events
            .into_iter()
            .filter_map(|event| project_activity(&event))
            .collect::<Vec<_>>();

        if let Some(page_token) = filter.page_token {
            if let Some(position) = activities
                .iter()
                .position(|activity| activity.id == page_token)
            {
                activities = activities.into_iter().skip(position + 1).collect();
            }
        }

        if let Some(page_size) = filter.page_size {
            activities.truncate(page_size as usize);
        }

        activities
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

        let leg_requests = current
            .order
            .legs
            .as_deref()
            .map(option_leg_requests_from_orders)
            .unwrap_or_default();
        let market_quotes = self
            .resolve_market_quotes(
                if current.order.order_class == OrderClass::Mleg || current.order.symbol.is_empty()
                {
                    None
                } else {
                    Some(current.order.symbol.as_str())
                },
                if leg_requests.is_empty() {
                    None
                } else {
                    Some(leg_requests.as_slice())
                },
            )
            .await?;
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
        let qty = if current.order.order_class == OrderClass::Mleg {
            normalize_qty(replacement_qty, None, Decimal::ONE)?
        } else {
            let snapshot = market_quotes.get(&current.order.symbol).ok_or_else(|| {
                MockStateError::MarketDataUnavailable(format!(
                    "mock order replacement for {} requires live market data",
                    current.order.symbol
                ))
            })?;
            normalize_qty(
                replacement_qty,
                current.order.notional,
                reference_price(&request_side, snapshot),
            )?
        };

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
        let replacement_time_in_force_for_legs = replacement_time_in_force.clone();
        let mut replacement = Order {
            id: replacement_order_id.clone(),
            client_order_id: replacement_client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            filled_at: None,
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
            filled_qty: Decimal::ZERO,
            filled_avg_price: None,
            order_class: current.order.order_class.clone(),
            order_type: current.order.order_type.clone(),
            r#type: current.order.r#type.clone(),
            side: current.order.side.clone(),
            position_intent: current.order.position_intent.clone(),
            time_in_force: replacement_time_in_force,
            limit_price: replacement_limit_price,
            stop_price: input.stop_price.or(current.order.stop_price),
            status: OrderStatus::New,
            extended_hours: current.order.extended_hours,
            legs: build_order_legs(
                &current.order.order_class,
                qty,
                current.order.r#type.clone(),
                replacement_time_in_force_for_legs,
                &now,
                &current.order.symbol,
                &market_quotes,
                &request_side,
                current.order.position_intent.clone(),
                current.order.take_profit.clone(),
                current.order.stop_loss.clone(),
                if leg_requests.is_empty() {
                    None
                } else {
                    Some(leg_requests.as_slice())
                },
                current.order.legs.as_deref(),
            )?,
            trail_percent: current.order.trail_percent,
            trail_price: input.trail.or(current.order.trail_price),
            hwm: current.order.hwm,
            ratio_qty: current.order.ratio_qty,
            take_profit: current.order.take_profit.clone(),
            stop_loss: current.order.stop_loss.clone(),
            subtag: current.order.subtag.clone(),
            source: current.order.source.clone(),
        };
        if current.order.order_class == OrderClass::Mleg {
            apply_mleg_fill_rules(&mut replacement, &request_side, &market_quotes);
        } else {
            let snapshot = market_quotes
                .get(&current.order.symbol)
                .expect("simple order market quote should exist");
            let fill_price = marketable_fill_price(
                &current.order.r#type,
                &request_side,
                replacement.limit_price,
                snapshot,
            );
            replacement.filled_at = fill_price.map(|_| now.clone());
            replacement.filled_qty = fill_price.map_or(Decimal::ZERO, |_| qty);
            replacement.filled_avg_price = fill_price;
            replacement.status = if fill_price.is_some() {
                OrderStatus::Filled
            } else {
                OrderStatus::New
            };
        }

        let (current_order_id, current_client_order_id, current_symbol, current_asset_class) = {
            let current = account.orders.get_mut(order_id).ok_or_else(|| {
                MockStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            if is_terminal_status(&current.order.status) {
                return Err(MockStateError::Conflict(format!(
                    "order {order_id} is no longer replaceable"
                )));
            }
            mark_order_replaced(&mut current.order, &replacement, &now);
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
            mark_order_canceled(&mut stored.order, &now);
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
                mark_order_canceled(&mut stored.order, &now);
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
        self.inner
            .market_data_cache
            .write()
            .expect("market data cache lock should not poison")
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
    pub fn take_http_fault(&self) -> Option<InjectedHttpFault> {
        self.inner
            .http_fault
            .write()
            .expect("fault lock should not poison")
            .take()
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
        if let Some(snapshot) = self
            .inner
            .market_data_cache
            .read()
            .expect("market data cache lock should not poison")
            .get(symbol)
            .cloned()
        {
            return Ok(snapshot);
        }

        let bridge = self.market_data_bridge().cloned().ok_or_else(|| {
            MockStateError::MarketDataUnavailable(
                "mock order simulation requires ALPACA_DATA_* credentials and a configured market data bridge".to_owned(),
            )
        })?;
        let snapshot = bridge
            .instrument_snapshot(symbol)
            .await
            .map_err(|error| MockStateError::MarketDataUnavailable(error.to_string()))?;

        self.inner
            .market_data_cache
            .write()
            .expect("market data cache lock should not poison")
            .insert(symbol.to_owned(), snapshot.clone());

        Ok(snapshot)
    }

    async fn resolve_market_quotes(
        &self,
        symbol: Option<&str>,
        legs: Option<&[OptionLegRequest]>,
    ) -> Result<HashMap<String, InstrumentSnapshot>, MockStateError> {
        let mut quotes = HashMap::new();
        for requested_symbol in requested_symbols(symbol, legs) {
            let snapshot = self.instrument_snapshot(&requested_symbol).await?;
            quotes.insert(requested_symbol, snapshot);
        }
        Ok(quotes)
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
        OrderSide::Buy | OrderSide::Sell => snapshot.mid_price(),
        OrderSide::Unspecified => snapshot.mid_price(),
    }
}

fn marketable_fill_price(
    order_type: &OrderType,
    side: &OrderSide,
    limit_price: Option<Decimal>,
    snapshot: &InstrumentSnapshot,
) -> Option<Decimal> {
    let mid = snapshot.mid_price();
    match order_type {
        OrderType::Market => Some(reference_price(side, snapshot)),
        OrderType::Limit => match side {
            OrderSide::Buy => limit_price.filter(|limit| *limit >= mid).map(|_| mid),
            OrderSide::Sell => limit_price.filter(|limit| *limit <= mid).map(|_| mid),
            OrderSide::Unspecified => None,
        },
        OrderType::Stop
        | OrderType::StopLimit
        | OrderType::TrailingStop
        | OrderType::Unspecified => None,
    }
}

fn apply_mleg_fill_rules(
    order: &mut Order,
    request_side: &OrderSide,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) {
    let mid = mleg_mid_price(order, request_side, market_quotes);
    let fill_price = mid.and_then(|mid| match order.r#type {
        OrderType::Market => Some(mid),
        OrderType::Limit => match request_side {
            OrderSide::Buy | OrderSide::Unspecified => order
                .limit_price
                .filter(|limit_price| *limit_price >= mid)
                .map(|_| mid),
            OrderSide::Sell => order
                .limit_price
                .filter(|limit_price| *limit_price <= mid)
                .map(|_| mid),
        },
        OrderType::Stop
        | OrderType::StopLimit
        | OrderType::TrailingStop
        | OrderType::Unspecified => None,
    });

    if let Some(fill_price) = fill_price {
        let now = now_string();
        order.status = OrderStatus::Filled;
        order.filled_qty = order.qty.unwrap_or(Decimal::ZERO);
        order.filled_avg_price = Some(fill_price);
        order.filled_at = Some(now.clone());
        order.updated_at = now;
        order.canceled_at = None;
        sync_nested_legs(order, market_quotes, Some(fill_price), OrderStatus::Filled);
        return;
    }

    order.status = OrderStatus::New;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;
    order.filled_at = None;
    sync_nested_legs(order, market_quotes, None, OrderStatus::New);
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
    let occurred_at = order
        .filled_at
        .clone()
        .unwrap_or_else(|| order.updated_at.clone());
    let executions = execution_facts_from_order(account, order, request_side, &occurred_at);
    for execution in executions {
        account.positions.apply_execution(&execution);
        account.executions.push(execution);
    }
    let activity_sequence = account.next_sequence();
    account.activities.push(
        ActivityEvent::new(
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
        )
        .with_fill_order(order, request_side),
    );
}

fn signed_cash_delta(side: &OrderSide, qty: Decimal, price: Decimal) -> Decimal {
    let gross = (price * qty).round_dp(8);
    match side {
        OrderSide::Buy => -gross,
        OrderSide::Sell => gross,
        OrderSide::Unspecified => Decimal::ZERO,
    }
}

fn execution_facts_from_order(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
    occurred_at: &str,
) -> Vec<ExecutionFact> {
    if order.order_class == OrderClass::Mleg {
        return order
            .legs
            .as_ref()
            .map(|legs| {
                legs.iter()
                    .map(|leg| {
                        ExecutionFact::new(
                            account.next_sequence(),
                            leg.id.clone(),
                            leg.asset_id.clone(),
                            leg.symbol.clone(),
                            leg.asset_class.clone(),
                            leg.side.clone(),
                            leg.position_intent.clone(),
                            leg.filled_qty,
                            leg.filled_avg_price.unwrap_or(Decimal::ZERO),
                            leg.filled_at
                                .clone()
                                .unwrap_or_else(|| occurred_at.to_owned()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    vec![ExecutionFact::new(
        account.next_sequence(),
        order.id.clone(),
        order.asset_id.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        request_side.clone(),
        order.position_intent.clone(),
        order.filled_qty,
        order.filled_avg_price.unwrap_or(Decimal::ZERO),
        occurred_at.to_owned(),
    )]
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

fn order_for_list(order: &Order, nested: bool) -> Order {
    if nested || order.order_class == OrderClass::Mleg {
        return order.clone();
    }

    let mut projected = order.clone();
    if matches!(
        projected.order_class,
        OrderClass::Bracket | OrderClass::Oco | OrderClass::Oto
    ) {
        projected.legs = None;
    }
    projected
}

fn event_matches_date(event: &ActivityEvent, date: &str) -> bool {
    event
        .occurred_at
        .split_once('T')
        .map(|(event_date, _)| event_date == date)
        .unwrap_or_else(|| event.occurred_at.starts_with(date))
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

fn mleg_mid_price(
    order: &Order,
    request_side: &OrderSide,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<Decimal> {
    let raw_total = mleg_price_total(
        order,
        market_quotes,
        |instrument, leg_side| match leg_side {
            OrderSide::Buy | OrderSide::Sell => Some(instrument.mid_price()),
            OrderSide::Unspecified => None,
        },
    )?;

    let normalized_total = match request_side {
        OrderSide::Buy | OrderSide::Unspecified => raw_total,
        OrderSide::Sell => -raw_total,
    };

    Some(normalized_total.round_dp(2))
}

fn mleg_price_total<F>(
    order: &Order,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
    mut price_for_leg: F,
) -> Option<Decimal>
where
    F: FnMut(&InstrumentSnapshot, &OrderSide) -> Option<Decimal>,
{
    order
        .legs
        .as_ref()?
        .iter()
        .try_fold(Decimal::ZERO, |total, leg| {
            let instrument = market_quotes.get(&leg.symbol)?;
            let leg_price = price_for_leg(instrument, &leg.side)?;
            let ratio_qty = Decimal::from(leg.ratio_qty.unwrap_or(1));
            let contribution = match leg.side {
                OrderSide::Buy => leg_price * ratio_qty,
                OrderSide::Sell => -(leg_price * ratio_qty),
                OrderSide::Unspecified => return None,
            };
            Some(total + contribution)
        })
}

fn sync_nested_legs(
    order: &mut Order,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
    fill_price: Option<Decimal>,
    status: OrderStatus,
) {
    let Some(legs) = order.legs.as_mut() else {
        return;
    };

    let now = now_string();
    for leg in legs {
        leg.updated_at = now.clone();
        leg.status = status.clone();
        match fill_price {
            Some(_) => {
                leg.filled_qty = leg.qty.unwrap_or(Decimal::ZERO);
                leg.filled_avg_price = market_quotes
                    .get(&leg.symbol)
                    .map(InstrumentSnapshot::mid_price);
                leg.filled_at = Some(now.clone());
                leg.canceled_at = None;
            }
            None => {
                leg.filled_qty = Decimal::ZERO;
                leg.filled_avg_price = None;
                leg.filled_at = None;
            }
        }
    }
}

fn mark_order_canceled(order: &mut Order, canceled_at: &str) {
    order.status = OrderStatus::Canceled;
    order.updated_at = canceled_at.to_owned();
    order.canceled_at = Some(canceled_at.to_owned());
    order.filled_at = None;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;

    if let Some(legs) = order.legs.as_mut() {
        for leg in legs {
            leg.status = OrderStatus::Canceled;
            leg.updated_at = canceled_at.to_owned();
            leg.canceled_at = Some(canceled_at.to_owned());
            leg.filled_at = None;
            leg.filled_qty = Decimal::ZERO;
            leg.filled_avg_price = None;
        }
    }
}

fn mark_order_replaced(order: &mut Order, replacement: &Order, replaced_at: &str) {
    order.status = OrderStatus::Replaced;
    order.updated_at = replaced_at.to_owned();
    order.replaced_at = Some(replaced_at.to_owned());
    order.replaced_by = Some(replacement.id.clone());

    if let (Some(current_legs), Some(replacement_legs)) =
        (order.legs.as_mut(), replacement.legs.as_ref())
    {
        for (current_leg, replacement_leg) in current_legs.iter_mut().zip(replacement_legs.iter()) {
            current_leg.status = OrderStatus::Replaced;
            current_leg.updated_at = replaced_at.to_owned();
            current_leg.replaced_at = Some(replaced_at.to_owned());
            current_leg.replaced_by = Some(replacement_leg.id.clone());
        }
    }
}

fn requested_symbols(symbol: Option<&str>, legs: Option<&[OptionLegRequest]>) -> Vec<String> {
    let mut symbols = Vec::new();
    if let Some(symbol) = symbol {
        symbols.push(symbol.to_owned());
    }
    if let Some(legs) = legs {
        symbols.extend(legs.iter().map(|leg| leg.symbol.clone()));
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

fn option_leg_requests_from_orders(legs: &[Order]) -> Vec<OptionLegRequest> {
    legs.iter()
        .map(|leg| OptionLegRequest {
            symbol: leg.symbol.clone(),
            ratio_qty: leg.ratio_qty.unwrap_or(1),
            side: Some(leg.side.clone()),
            position_intent: leg.position_intent.clone(),
        })
        .collect()
}

fn build_order_legs(
    order_class: &OrderClass,
    parent_qty: Decimal,
    order_type: OrderType,
    time_in_force: TimeInForce,
    now: &str,
    symbol: &str,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
    request_side: &OrderSide,
    position_intent: Option<PositionIntent>,
    take_profit: Option<TakeProfit>,
    stop_loss: Option<StopLoss>,
    mleg_legs: Option<&[OptionLegRequest]>,
    previous_legs: Option<&[Order]>,
) -> Result<Option<Vec<Order>>, MockStateError> {
    match order_class {
        OrderClass::Simple => Ok(None),
        OrderClass::Mleg => Ok(Some(build_leg_orders_from_requests(
            mleg_legs.unwrap_or(&[]),
            parent_qty,
            order_type,
            time_in_force,
            now,
            previous_legs,
        ))),
        OrderClass::Bracket | OrderClass::Oco | OrderClass::Oto => {
            Ok(Some(build_advanced_order_legs(
                order_class,
                parent_qty,
                time_in_force,
                now,
                symbol,
                market_quotes,
                request_side,
                position_intent,
                take_profit,
                stop_loss,
                previous_legs,
            )?))
        }
    }
}

fn build_advanced_order_legs(
    order_class: &OrderClass,
    parent_qty: Decimal,
    time_in_force: TimeInForce,
    now: &str,
    symbol: &str,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
    request_side: &OrderSide,
    position_intent: Option<PositionIntent>,
    take_profit: Option<TakeProfit>,
    stop_loss: Option<StopLoss>,
    previous_legs: Option<&[Order]>,
) -> Result<Vec<Order>, MockStateError> {
    let instrument = market_quotes.get(symbol).ok_or_else(|| {
        MockStateError::MarketDataUnavailable(format!(
            "mock advanced order for {symbol} requires live market data"
        ))
    })?;
    let child_side = advanced_child_side(order_class, request_side);
    let child_position_intent = advanced_child_position_intent(&position_intent, &child_side);
    let mut children = Vec::new();

    match order_class {
        OrderClass::Bracket => {
            let take_profit = take_profit.ok_or_else(|| {
                MockStateError::Conflict(
                    "bracket orders require a take_profit configuration".to_owned(),
                )
            })?;
            let stop_loss = stop_loss.ok_or_else(|| {
                MockStateError::Conflict(
                    "bracket orders require a stop_loss configuration".to_owned(),
                )
            })?;
            children.push(build_advanced_child_order(
                symbol,
                &instrument.asset_class,
                child_side.clone(),
                child_position_intent.clone(),
                parent_qty,
                OrderType::Limit,
                time_in_force.clone(),
                now,
                Some(take_profit.limit_price),
                None,
                previous_legs.and_then(|legs| legs.first()),
            ));
            children.push(build_advanced_child_order(
                symbol,
                &instrument.asset_class,
                child_side,
                child_position_intent,
                parent_qty,
                stop_order_type(&stop_loss),
                time_in_force,
                now,
                stop_loss.limit_price,
                Some(stop_loss.stop_price),
                previous_legs.and_then(|legs| legs.get(1)),
            ));
        }
        OrderClass::Oto => {
            if let Some(take_profit) = take_profit {
                children.push(build_advanced_child_order(
                    symbol,
                    &instrument.asset_class,
                    child_side.clone(),
                    child_position_intent.clone(),
                    parent_qty,
                    OrderType::Limit,
                    time_in_force.clone(),
                    now,
                    Some(take_profit.limit_price),
                    None,
                    previous_legs.and_then(|legs| legs.first()),
                ));
            } else if let Some(stop_loss) = stop_loss {
                children.push(build_advanced_child_order(
                    symbol,
                    &instrument.asset_class,
                    child_side,
                    child_position_intent,
                    parent_qty,
                    stop_order_type(&stop_loss),
                    time_in_force,
                    now,
                    stop_loss.limit_price,
                    Some(stop_loss.stop_price),
                    previous_legs.and_then(|legs| legs.first()),
                ));
            } else {
                return Err(MockStateError::Conflict(
                    "oto orders require a take_profit or stop_loss configuration".to_owned(),
                ));
            }
        }
        OrderClass::Oco => {
            let stop_loss = stop_loss.ok_or_else(|| {
                MockStateError::Conflict("oco orders require a stop_loss configuration".to_owned())
            })?;
            let _ = take_profit.ok_or_else(|| {
                MockStateError::Conflict(
                    "oco orders require a take_profit configuration".to_owned(),
                )
            })?;
            children.push(build_advanced_child_order(
                symbol,
                &instrument.asset_class,
                child_side,
                child_position_intent,
                parent_qty,
                stop_order_type(&stop_loss),
                time_in_force,
                now,
                stop_loss.limit_price,
                Some(stop_loss.stop_price),
                previous_legs.and_then(|legs| legs.first()),
            ));
        }
        OrderClass::Simple | OrderClass::Mleg => {}
    }

    Ok(children)
}

fn build_advanced_child_order(
    symbol: &str,
    asset_class: &str,
    side: OrderSide,
    position_intent: Option<PositionIntent>,
    qty: Decimal,
    order_type: OrderType,
    time_in_force: TimeInForce,
    now: &str,
    limit_price: Option<Decimal>,
    stop_price: Option<Decimal>,
    previous_leg: Option<&Order>,
) -> Order {
    Order {
        id: format!("mock-child-order-{}-{}", now_millis(), symbol),
        client_order_id: format!("mock-child-client-order-{}-{}", now_millis(), symbol),
        created_at: now.to_owned(),
        updated_at: now.to_owned(),
        submitted_at: now.to_owned(),
        filled_at: None,
        expired_at: None,
        expires_at: expires_at_for(&time_in_force),
        canceled_at: None,
        failed_at: None,
        replaced_at: None,
        replaced_by: None,
        replaces: previous_leg.map(|leg| leg.id.clone()),
        asset_id: previous_leg
            .map(|leg| leg.asset_id.clone())
            .unwrap_or_else(|| mock_asset_id(symbol)),
        symbol: symbol.to_owned(),
        asset_class: asset_class.to_owned(),
        notional: None,
        qty: Some(qty),
        filled_qty: Decimal::ZERO,
        filled_avg_price: None,
        order_class: OrderClass::Simple,
        order_type: order_type.clone(),
        r#type: order_type,
        side,
        position_intent,
        time_in_force,
        limit_price,
        stop_price,
        status: OrderStatus::New,
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
    }
}

fn advanced_child_side(order_class: &OrderClass, request_side: &OrderSide) -> OrderSide {
    match order_class {
        OrderClass::Oco => request_side.clone(),
        OrderClass::Bracket | OrderClass::Oto => match request_side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
            OrderSide::Unspecified => OrderSide::Unspecified,
        },
        OrderClass::Simple | OrderClass::Mleg => request_side.clone(),
    }
}

fn advanced_child_position_intent(
    parent_position_intent: &Option<PositionIntent>,
    child_side: &OrderSide,
) -> Option<PositionIntent> {
    match (parent_position_intent.as_ref(), child_side) {
        (Some(PositionIntent::BuyToOpen | PositionIntent::SellToOpen), OrderSide::Buy) => {
            Some(PositionIntent::BuyToClose)
        }
        (Some(PositionIntent::BuyToOpen | PositionIntent::SellToOpen), OrderSide::Sell) => {
            Some(PositionIntent::SellToClose)
        }
        _ => None,
    }
}

fn stop_order_type(stop_loss: &StopLoss) -> OrderType {
    if stop_loss.limit_price.is_some() {
        OrderType::StopLimit
    } else {
        OrderType::Stop
    }
}

fn build_leg_orders_from_requests(
    legs: &[OptionLegRequest],
    parent_qty: Decimal,
    order_type: OrderType,
    time_in_force: TimeInForce,
    now: &str,
    previous_legs: Option<&[Order]>,
) -> Vec<Order> {
    legs.iter()
        .enumerate()
        .map(|(index, leg)| {
            let previous_leg = previous_legs.and_then(|legs| legs.get(index));
            let leg_qty = parent_qty * Decimal::from(leg.ratio_qty);
            Order {
                id: format!("mock-leg-order-{}-{index}", now_millis()),
                client_order_id: format!("mock-leg-client-order-{}-{index}", now_millis()),
                created_at: now.to_owned(),
                updated_at: now.to_owned(),
                submitted_at: now.to_owned(),
                filled_at: None,
                expired_at: None,
                expires_at: expires_at_for(&time_in_force),
                canceled_at: None,
                failed_at: None,
                replaced_at: None,
                replaced_by: None,
                replaces: previous_leg.map(|leg| leg.id.clone()),
                asset_id: previous_leg
                    .map(|leg| leg.asset_id.clone())
                    .unwrap_or_else(|| mock_asset_id(&leg.symbol)),
                symbol: leg.symbol.clone(),
                asset_class: "us_option".to_owned(),
                notional: None,
                qty: Some(leg_qty),
                filled_qty: Decimal::ZERO,
                filled_avg_price: None,
                order_class: OrderClass::Mleg,
                order_type: order_type.clone(),
                r#type: order_type.clone(),
                side: leg.side.clone().unwrap_or(OrderSide::Buy),
                position_intent: leg.position_intent.clone(),
                time_in_force: time_in_force.clone(),
                limit_price: None,
                stop_price: None,
                status: OrderStatus::New,
                extended_hours: false,
                legs: None,
                trail_percent: None,
                trail_price: None,
                hwm: None,
                ratio_qty: Some(leg.ratio_qty),
                take_profit: None,
                stop_loss: None,
                subtag: None,
                source: None,
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn equity_snapshot() -> InstrumentSnapshot {
        InstrumentSnapshot::equity(Decimal::new(100, 0), Decimal::new(101, 0))
    }

    fn option_snapshot() -> InstrumentSnapshot {
        InstrumentSnapshot::option(Decimal::new(120, 2), Decimal::new(140, 2))
    }

    fn build_test_mleg_order(order_type: OrderType, limit_price: Option<Decimal>) -> Order {
        let now = "2026-04-09T12:00:00Z";
        Order {
            id: "mock-parent-order".to_owned(),
            client_order_id: "mock-parent-client-order".to_owned(),
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
            submitted_at: now.to_owned(),
            filled_at: None,
            expired_at: None,
            expires_at: expires_at_for(&TimeInForce::Day),
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: None,
            asset_id: String::new(),
            symbol: String::new(),
            asset_class: String::new(),
            notional: None,
            qty: Some(Decimal::ONE),
            filled_qty: Decimal::ZERO,
            filled_avg_price: None,
            order_class: OrderClass::Mleg,
            order_type: order_type.clone(),
            r#type: order_type.clone(),
            side: OrderSide::Unspecified,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price,
            stop_price: None,
            status: OrderStatus::New,
            extended_hours: false,
            legs: Some(build_leg_orders_from_requests(
                &[
                    OptionLegRequest {
                        symbol: "OPT-BUY".to_owned(),
                        ratio_qty: 1,
                        side: Some(OrderSide::Buy),
                        position_intent: None,
                    },
                    OptionLegRequest {
                        symbol: "OPT-SELL".to_owned(),
                        ratio_qty: 1,
                        side: Some(OrderSide::Sell),
                        position_intent: None,
                    },
                ],
                Decimal::ONE,
                order_type,
                TimeInForce::Day,
                now,
                None,
            )),
            trail_percent: None,
            trail_price: None,
            hwm: None,
            ratio_qty: None,
            take_profit: None,
            stop_loss: None,
            subtag: None,
            source: None,
        }
    }

    #[test]
    fn stock_single_leg_orders_use_mid_price_for_market_and_limit() {
        let snapshot = equity_snapshot();
        let mid = snapshot.mid_price();

        assert_eq!(reference_price(&OrderSide::Buy, &snapshot), mid);
        assert_eq!(reference_price(&OrderSide::Sell, &snapshot), mid);
        assert_eq!(
            marketable_fill_price(&OrderType::Market, &OrderSide::Buy, None, &snapshot),
            Some(mid)
        );
        assert_eq!(
            marketable_fill_price(&OrderType::Limit, &OrderSide::Buy, Some(mid), &snapshot,),
            Some(mid)
        );
        assert_eq!(
            marketable_fill_price(
                &OrderType::Limit,
                &OrderSide::Buy,
                Some(mid - Decimal::new(1, 2)),
                &snapshot,
            ),
            None
        );
        assert_eq!(
            marketable_fill_price(&OrderType::Limit, &OrderSide::Sell, Some(mid), &snapshot,),
            Some(mid)
        );
        assert_eq!(
            marketable_fill_price(
                &OrderType::Limit,
                &OrderSide::Sell,
                Some(mid + Decimal::new(1, 2)),
                &snapshot,
            ),
            None
        );
    }

    #[test]
    fn option_single_leg_orders_use_mid_price_for_market_and_limit() {
        let snapshot = option_snapshot();
        let mid = snapshot.mid_price();

        assert_eq!(reference_price(&OrderSide::Buy, &snapshot), mid);
        assert_eq!(reference_price(&OrderSide::Sell, &snapshot), mid);
        assert_eq!(
            marketable_fill_price(&OrderType::Market, &OrderSide::Sell, None, &snapshot),
            Some(mid)
        );
        assert_eq!(
            marketable_fill_price(&OrderType::Limit, &OrderSide::Buy, Some(mid), &snapshot,),
            Some(mid)
        );
        assert_eq!(
            marketable_fill_price(&OrderType::Limit, &OrderSide::Sell, Some(mid), &snapshot,),
            Some(mid)
        );
    }

    #[test]
    fn multi_leg_orders_use_composite_mid_price_for_market_and_limit() {
        let market_quotes = HashMap::from([
            (
                "OPT-BUY".to_owned(),
                InstrumentSnapshot::option(Decimal::new(300, 2), Decimal::new(340, 2)),
            ),
            (
                "OPT-SELL".to_owned(),
                InstrumentSnapshot::option(Decimal::new(100, 2), Decimal::new(140, 2)),
            ),
        ]);
        let expected_mid = Decimal::new(200, 2);

        let mut market_order = build_test_mleg_order(OrderType::Market, None);
        apply_mleg_fill_rules(&mut market_order, &OrderSide::Buy, &market_quotes);
        assert_eq!(market_order.status, OrderStatus::Filled);
        assert_eq!(market_order.filled_avg_price, Some(expected_mid));

        let filled_legs = market_order
            .legs
            .expect("filled mleg should keep nested legs");
        assert_eq!(filled_legs.len(), 2);
        assert_eq!(filled_legs[0].filled_avg_price, Some(Decimal::new(320, 2)));
        assert_eq!(filled_legs[1].filled_avg_price, Some(Decimal::new(120, 2)));

        let mut limit_order = build_test_mleg_order(OrderType::Limit, Some(expected_mid));
        apply_mleg_fill_rules(&mut limit_order, &OrderSide::Buy, &market_quotes);
        assert_eq!(limit_order.status, OrderStatus::Filled);
        assert_eq!(limit_order.filled_avg_price, Some(expected_mid));

        let mut resting_order =
            build_test_mleg_order(OrderType::Limit, Some(expected_mid - Decimal::new(1, 2)));
        apply_mleg_fill_rules(&mut resting_order, &OrderSide::Buy, &market_quotes);
        assert_eq!(resting_order.status, OrderStatus::New);
        assert_eq!(resting_order.filled_avg_price, None);

        let mut sell_limit_order = build_test_mleg_order(OrderType::Limit, Some(-expected_mid));
        apply_mleg_fill_rules(&mut sell_limit_order, &OrderSide::Sell, &market_quotes);
        assert_eq!(sell_limit_order.status, OrderStatus::Filled);
        assert_eq!(sell_limit_order.filled_avg_price, Some(-expected_mid));

        let mut sell_resting_order = build_test_mleg_order(
            OrderType::Limit,
            Some(-expected_mid + Decimal::new(1, 2)),
        );
        apply_mleg_fill_rules(&mut sell_resting_order, &OrderSide::Sell, &market_quotes);
        assert_eq!(sell_resting_order.status, OrderStatus::New);
        assert_eq!(sell_resting_order.filled_avg_price, None);
    }

    #[test]
    fn filled_multi_leg_short_legs_project_negative_qty() {
        let market_quotes = HashMap::from([
            (
                "OPT-LONG".to_owned(),
                InstrumentSnapshot::option(Decimal::new(300, 2), Decimal::new(340, 2)),
            ),
            (
                "OPT-SHORT".to_owned(),
                InstrumentSnapshot::option(Decimal::new(100, 2), Decimal::new(140, 2)),
            ),
        ]);
        let now = "2026-04-09T12:00:00Z";
        let mut order = Order {
            id: "mock-parent-order".to_owned(),
            client_order_id: "mock-parent-client-order".to_owned(),
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
            submitted_at: now.to_owned(),
            filled_at: None,
            expired_at: None,
            expires_at: expires_at_for(&TimeInForce::Day),
            canceled_at: None,
            failed_at: None,
            replaced_at: None,
            replaced_by: None,
            replaces: None,
            asset_id: String::new(),
            symbol: String::new(),
            asset_class: String::new(),
            notional: None,
            qty: Some(Decimal::new(2, 0)),
            filled_qty: Decimal::ZERO,
            filled_avg_price: None,
            order_class: OrderClass::Mleg,
            order_type: OrderType::Market,
            r#type: OrderType::Market,
            side: OrderSide::Unspecified,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: None,
            stop_price: None,
            status: OrderStatus::New,
            extended_hours: false,
            legs: Some(build_leg_orders_from_requests(
                &[
                    OptionLegRequest {
                        symbol: "OPT-LONG".to_owned(),
                        ratio_qty: 1,
                        side: Some(OrderSide::Buy),
                        position_intent: Some(PositionIntent::BuyToOpen),
                    },
                    OptionLegRequest {
                        symbol: "OPT-SHORT".to_owned(),
                        ratio_qty: 2,
                        side: Some(OrderSide::Sell),
                        position_intent: Some(PositionIntent::SellToOpen),
                    },
                ],
                Decimal::new(2, 0),
                OrderType::Market,
                TimeInForce::Day,
                now,
                None,
            )),
            trail_percent: None,
            trail_price: None,
            hwm: None,
            ratio_qty: None,
            take_profit: None,
            stop_loss: None,
            subtag: None,
            source: None,
        };

        apply_mleg_fill_rules(&mut order, &OrderSide::Buy, &market_quotes);
        assert_eq!(order.status, OrderStatus::Filled);

        let mut account = VirtualAccountState::new("mock-key");
        apply_fill_effects(&mut account, &order, &OrderSide::Buy);

        let positions = account.positions.list_open_positions();
        let short_position = positions
            .iter()
            .find(|position| position.instrument_identity.symbol == "OPT-SHORT")
            .expect("short leg position should exist");
        let projected = project_position(
            short_position,
            market_quotes
                .get("OPT-SHORT")
                .expect("short market quote should exist"),
        );

        assert_eq!(short_position.net_qty, Decimal::new(-4, 0));
        assert_eq!(projected.qty, Decimal::new(-4, 0));
        assert_eq!(projected.side, "short");
    }
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
