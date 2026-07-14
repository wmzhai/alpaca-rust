use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

mod account;
mod activities;
mod assets;
mod calendar;
mod clock;
mod executions;
mod market_data;
mod options_contracts;
mod positions;
mod watchlists;

use chrono::{SecondsFormat, Utc};
use chrono_tz::America::New_York;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::Serialize;
use thiserror::Error;

use alpaca_option::{
    OptionContract, OptionQuote, OrderSide as QuoteOrderSide, QuotedLeg, execution_quote,
};
use alpaca_trade::account_configurations::{AccountConfigurations, UpdateRequest};
use alpaca_trade::activities::{Activity, ActivityCategory};
use alpaca_trade::assets::{Asset, AssetAttribute, AssetClass, AssetStatus, Exchange};
use alpaca_trade::calendar::{Calendar, CalendarTimezone, CalendarV3Response, DateType, Market};
use alpaca_trade::clock::{Clock, ClockV3Response};
use alpaca_trade::options_contracts::{
    ContractStatus, ContractStyle, ContractType, ListResponse,
    OptionContract as TradeOptionContract,
};
use alpaca_trade::orders::{
    AdvancedInstructions, CancelAllOrderResult, OptionLegRequest, Order, OrderAssetClass,
    OrderClass, OrderLeg, OrderSide, OrderStatus, OrderType, PositionIntent, QueryOrderStatus,
    SortDirection, StopLoss, TakeProfit, TimeInForce,
};
use alpaca_trade::portfolio_history::PortfolioHistory;
use alpaca_trade::positions::{
    ClosePositionResult, ExerciseDetails, Position, PositionExchange,
    PositionSide as TradePositionSide,
};
use alpaca_trade::watchlists::{Watchlist, WatchlistSummary};

use activities::{
    ActivityEvent, ActivityEventKind, is_public_activity, matches_activity_type, project_activity,
};
use executions::ExecutionFact;
pub use market_data::{DEFAULT_STOCK_SYMBOL, InstrumentSnapshot, LiveMarketDataBridge};
use positions::{OptionContractType, PositionBook, parse_option_symbol, project_position};
use watchlists::WatchlistBook;

#[derive(Debug, Clone)]
pub struct MockServerState {
    inner: Arc<SharedState>,
}

#[derive(Debug)]
struct SharedState {
    accounts: RwLock<HashMap<String, VirtualAccountState>>,
    http_fault: RwLock<Option<InjectedHttpFault>>,
    market_data_bridge: Option<LiveMarketDataBridge>,
    market_data_overrides: RwLock<HashMap<String, InstrumentSnapshot>>,
}

#[derive(Debug, Clone)]
pub(crate) struct VirtualAccountState {
    account_profile: account::AccountProfile,
    cash_ledger: account::CashLedger,
    account_configurations: AccountConfigurations,
    orders: HashMap<String, StoredOrder>,
    client_order_ids: HashMap<String, String>,
    executions: Vec<ExecutionFact>,
    positions: PositionBook,
    activities: Vec<ActivityEvent>,
    watchlists: WatchlistBook,
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
    pub advanced_instructions: Option<AdvancedInstructions>,
}

#[derive(Debug, Clone, Default)]
pub struct ReplaceOrderInput {
    pub qty: Option<Decimal>,
    pub time_in_force: Option<TimeInForce>,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trail: Option<Decimal>,
    pub client_order_id: Option<String>,
    pub advanced_instructions: Option<AdvancedInstructions>,
}

#[derive(Debug, Clone, Default)]
pub struct ClosePositionInput {
    pub qty: Option<Decimal>,
    pub percentage: Option<Decimal>,
}

#[derive(Debug, Clone, Default)]
pub struct ListOrdersFilter {
    pub status: Option<QueryOrderStatus>,
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub until: Option<String>,
    pub direction: Option<SortDirection>,
    pub symbols: Option<Vec<String>>,
    pub side: Option<OrderSide>,
    pub asset_classes: Option<Vec<OrderAssetClass>>,
    pub nested: Option<bool>,
    pub before_order_id: Option<String>,
    pub after_order_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ListActivitiesFilter {
    pub activity_types: Option<Vec<String>>,
    pub category: Option<ActivityCategory>,
    pub date: Option<String>,
    pub until: Option<String>,
    pub after: Option<String>,
    pub direction: Option<SortDirection>,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ListAssetsFilter {
    pub status: Option<AssetStatus>,
    pub asset_class: Option<AssetClass>,
    pub exchange: Option<Exchange>,
    pub attributes: Option<Vec<AssetAttribute>>,
}

#[derive(Debug, Clone, Default)]
pub struct ListOptionContractsFilter {
    pub underlying_symbols: Option<Vec<String>>,
    pub show_deliverables: Option<bool>,
    pub status: Option<ContractStatus>,
    pub expiration_date: Option<String>,
    pub expiration_date_gte: Option<String>,
    pub expiration_date_lte: Option<String>,
    pub root_symbol: Option<String>,
    pub contract_type: Option<ContractType>,
    pub style: Option<ContractStyle>,
    pub strike_price_gte: Option<Decimal>,
    pub strike_price_lte: Option<Decimal>,
    pub page_token: Option<String>,
    pub limit: Option<u32>,
    pub ppind: Option<bool>,
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
    Forbidden(String),
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
                market_data_overrides: RwLock::new(HashMap::new()),
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
    pub fn with_market_snapshot(mut self, symbol: &str, snapshot: InstrumentSnapshot) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("mock state should be uniquely owned during configuration")
            .market_data_overrides
            .get_mut()
            .expect("market data overrides lock should not poison")
            .insert(symbol.to_owned(), snapshot);
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

    #[must_use]
    pub fn project_account_configurations(&self, api_key: &str) -> AccountConfigurations {
        self.ensure_account(api_key);
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        accounts
            .get(api_key)
            .expect("account should exist after ensure_account")
            .account_configurations
            .clone()
    }

    pub fn update_account_configurations(
        &self,
        api_key: &str,
        request: UpdateRequest,
    ) -> AccountConfigurations {
        self.ensure_account(api_key);
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let configuration = &mut accounts
            .get_mut(api_key)
            .expect("account should exist after ensure_account")
            .account_configurations;

        if let Some(value) = request.trade_confirm_email {
            configuration.trade_confirm_email = Some(value);
        }
        if let Some(value) = request.suspend_trade {
            configuration.suspend_trade = Some(value);
        }
        if let Some(value) = request.no_shorting {
            configuration.no_shorting = Some(value);
        }
        if let Some(value) = request.fractional_trading {
            configuration.fractional_trading = Some(value);
        }
        if let Some(value) = request.max_margin_multiplier {
            configuration.max_margin_multiplier = Some(value);
        }
        if let Some(value) = request.max_options_trading_level {
            configuration.max_options_trading_level = Some(value);
        }
        if let Some(value) = request.ptp_no_exception_entry {
            configuration.ptp_no_exception_entry = Some(value);
        }
        if let Some(value) = request.disable_overnight_trading {
            configuration.disable_overnight_trading = Some(value);
        }

        configuration.clone()
    }

    #[must_use]
    pub fn project_portfolio_history(&self, api_key: &str, timeframe: &str) -> PortfolioHistory {
        self.ensure_account(api_key);
        let accounts = self
            .inner
            .accounts
            .read()
            .expect("accounts lock should not poison");
        let account = accounts
            .get(api_key)
            .expect("account should exist after ensure_account");
        let equity = cash_balance(account);

        PortfolioHistory {
            timestamp: vec![Utc::now().timestamp()],
            equity: vec![equity],
            profit_loss: vec![Decimal::ZERO],
            profit_loss_pct: vec![Decimal::ZERO],
            base_value: equity,
            base_value_asof: None,
            timeframe: timeframe.to_owned(),
            cashflow: None,
        }
    }

    #[must_use]
    pub fn list_watchlists(&self, api_key: &str) -> Vec<WatchlistSummary> {
        self.with_watchlists(api_key, |watchlists, _| watchlists.summaries())
    }

    pub fn create_watchlist(
        &self,
        api_key: &str,
        name: String,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, account_id| {
            watchlists.create(account_id, name, symbols)
        })
    }

    pub fn get_watchlist_by_id(
        &self,
        api_key: &str,
        watchlist_id: &str,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| watchlists.get_by_id(watchlist_id))
    }

    pub fn get_watchlist_by_name(
        &self,
        api_key: &str,
        name: &str,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| watchlists.get_by_name(name))
    }

    pub fn update_watchlist_by_id(
        &self,
        api_key: &str,
        watchlist_id: &str,
        name: Option<String>,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.update_by_id(watchlist_id, name, symbols)
        })
    }

    pub fn update_watchlist_by_name(
        &self,
        api_key: &str,
        current_name: &str,
        name: Option<String>,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.update_by_name(current_name, name, symbols)
        })
    }

    pub fn delete_watchlist_by_id(
        &self,
        api_key: &str,
        watchlist_id: &str,
    ) -> Result<(), MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.delete_by_id(watchlist_id)
        })
    }

    pub fn delete_watchlist_by_name(
        &self,
        api_key: &str,
        name: &str,
    ) -> Result<(), MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| watchlists.delete_by_name(name))
    }

    pub fn add_watchlist_asset_by_id(
        &self,
        api_key: &str,
        watchlist_id: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.add_asset_by_id(watchlist_id, symbol)
        })
    }

    pub fn add_watchlist_asset_by_name(
        &self,
        api_key: &str,
        name: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.add_asset_by_name(name, symbol)
        })
    }

    pub fn remove_watchlist_asset_by_id(
        &self,
        api_key: &str,
        watchlist_id: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        self.with_watchlists(api_key, |watchlists, _| {
            watchlists.remove_asset_by_id(watchlist_id, symbol)
        })
    }

    fn with_watchlists<T>(
        &self,
        api_key: &str,
        operation: impl FnOnce(&mut WatchlistBook, &str) -> T,
    ) -> T {
        let mut accounts = self
            .inner
            .accounts
            .write()
            .expect("accounts lock should not poison");
        let account = accounts
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key));
        let account_id = account.account_profile.id.clone();
        operation(&mut account.watchlists, &account_id)
    }

    pub async fn create_order(
        &self,
        api_key: &str,
        input: CreateOrderInput,
    ) -> Result<Order, MockStateError> {
        let order_class = input.order_class.clone().unwrap_or(OrderClass::Simple);
        let order_type = input.order_type.clone().unwrap_or(OrderType::Market);
        let time_in_force = input.time_in_force.clone().unwrap_or(TimeInForce::Day);
        validate_limit_price_for_order_class(&order_class, input.limit_price)?;
        let requested_symbol = input
            .symbol
            .clone()
            .unwrap_or_else(|| DEFAULT_STOCK_SYMBOL.to_owned());
        let requested_legs = input.legs.clone();
        let requested_position_intent = input.position_intent.clone();
        let requested_take_profit = input.take_profit.clone();
        let requested_stop_loss = input.stop_loss.clone();
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
        let request_side = if order_class == OrderClass::Mleg {
            infer_request_side(
                input.side.clone(),
                input.limit_price.clone(),
                requested_legs.as_deref(),
                &market_quotes,
            )
        } else {
            let side = input.side.clone().unwrap_or(OrderSide::Buy);
            if side == OrderSide::Unspecified {
                return Err(MockStateError::Conflict(
                    "mock orders require an explicit buy or sell side".to_owned(),
                ));
            }
            side
        };
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
        let client_order_id = input.client_order_id.unwrap_or_else(|| {
            format!(
                "mock-client-order-{}-{}",
                now_millis(),
                account.next_sequence()
            )
        });
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
            qty: input.notional.is_none().then_some(qty),
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
        record_create_effects(account, &order, &request_side, &market_quotes);

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
                .map(|symbol| symbol.trim().to_ascii_uppercase())
                .filter(|symbol| !symbol.is_empty())
                .collect::<HashSet<_>>()
        });

        let before_anchor = filter
            .before_order_id
            .as_ref()
            .and_then(|order_id| account.orders.get(order_id))
            .map(|stored| stored.order.clone());
        let after_anchor = filter
            .after_order_id
            .as_ref()
            .and_then(|order_id| account.orders.get(order_id))
            .map(|stored| stored.order.clone());
        let direction = filter.direction.unwrap_or(SortDirection::Desc);
        let limit = filter.limit.unwrap_or(50) as usize;
        let nested = filter.nested.unwrap_or(false);

        let mut orders =
            account
                .orders
                .values()
                .filter(|stored| {
                    let order = &stored.order;
                    matches_status_filter(order, filter.status)
                        && matches_order_symbols(
                            order,
                            symbol_filter.as_ref(),
                            filter.asset_classes.as_deref(),
                        )
                        && filter.side.as_ref().is_none_or(|side| &order.side == side)
                        && matches_order_asset_classes(order, filter.asset_classes.as_deref())
                        && filter.after.as_ref().is_none_or(|after| {
                            compare_timestamp(&order.submitted_at, after).is_gt()
                        })
                        && filter.until.as_ref().is_none_or(|until| {
                            compare_timestamp(&order.submitted_at, until).is_lt()
                        })
                        && before_anchor
                            .as_ref()
                            .is_none_or(|anchor| compare_order_submission(order, anchor).is_lt())
                        && after_anchor
                            .as_ref()
                            .is_none_or(|anchor| compare_order_submission(order, anchor).is_gt())
                })
                .map(|stored| &stored.order)
                .collect::<Vec<_>>();
        orders.sort_by(|left, right| compare_order_submission(left, right));
        if direction == SortDirection::Desc {
            orders.reverse();
        }
        orders
            .into_iter()
            .take(limit)
            .map(|order| order_for_list(order, nested))
            .collect()
    }

    #[must_use]
    pub fn get_order(&self, api_key: &str, order_id: &str, nested: bool) -> Option<Order> {
        self.inner
            .accounts
            .read()
            .expect("accounts lock should not poison")
            .get(api_key)
            .and_then(|account| account.orders.get(order_id))
            .map(|stored| order_for_list(&stored.order, nested))
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
        let category = filter.category;

        let mut events = account
            .activities
            .iter()
            .filter(|event| is_public_activity(event))
            .filter(|_| !matches!(category, Some(ActivityCategory::NonTradeActivity)))
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

    #[must_use]
    pub fn list_assets(&self, filter: ListAssetsFilter) -> Vec<Asset> {
        assets::catalog()
            .into_iter()
            .filter(|asset| filter.status.is_none_or(|value| asset.status == value))
            .filter(|asset| filter.asset_class.is_none_or(|value| asset.class == value))
            .filter(|asset| filter.exchange.is_none_or(|value| asset.exchange == value))
            .filter(|asset| {
                filter.attributes.as_ref().is_none_or(|requested| {
                    asset
                        .attributes
                        .as_ref()
                        .is_some_and(|actual| requested.iter().any(|value| actual.contains(value)))
                })
            })
            .collect()
    }

    #[must_use]
    pub fn get_asset(&self, symbol_or_asset_id: &str) -> Option<Asset> {
        assets::catalog().into_iter().find(|asset| {
            asset.id == symbol_or_asset_id || asset.symbol.eq_ignore_ascii_case(symbol_or_asset_id)
        })
    }

    #[must_use]
    pub fn list_option_contracts(&self, filter: ListOptionContractsFilter) -> ListResponse {
        if filter.page_token.is_some() {
            return ListResponse {
                option_contracts: Vec::new(),
                next_page_token: None,
            };
        }

        let mut contracts = options_contracts::catalog()
            .into_iter()
            .filter(|contract| {
                filter.underlying_symbols.as_ref().is_none_or(|symbols| {
                    symbols
                        .iter()
                        .any(|symbol| contract.underlying_symbol.eq_ignore_ascii_case(symbol))
                })
            })
            .filter(|contract| {
                filter
                    .status
                    .as_ref()
                    .is_none_or(|status| contract.status == *status)
            })
            .filter(|contract| {
                filter
                    .expiration_date
                    .as_ref()
                    .is_none_or(|date| contract.expiration_date == *date)
            })
            .filter(|contract| {
                filter
                    .expiration_date_gte
                    .as_ref()
                    .is_none_or(|date| contract.expiration_date >= *date)
            })
            .filter(|contract| {
                filter
                    .expiration_date_lte
                    .as_ref()
                    .is_none_or(|date| contract.expiration_date <= *date)
            })
            .filter(|contract| {
                filter
                    .root_symbol
                    .as_ref()
                    .is_none_or(|root| contract.root_symbol.as_ref() == Some(root))
            })
            .filter(|contract| {
                filter
                    .contract_type
                    .as_ref()
                    .is_none_or(|contract_type| contract.r#type == *contract_type)
            })
            .filter(|contract| {
                filter
                    .style
                    .as_ref()
                    .is_none_or(|style| contract.style == *style)
            })
            .filter(|contract| {
                filter
                    .strike_price_gte
                    .is_none_or(|value| contract.strike_price >= value)
            })
            .filter(|contract| {
                filter
                    .strike_price_lte
                    .is_none_or(|value| contract.strike_price <= value)
            })
            .filter(|contract| {
                filter
                    .ppind
                    .is_none_or(|value| contract.ppind == Some(value))
            })
            .map(|mut contract| {
                if filter.show_deliverables != Some(true) {
                    contract.deliverables = None;
                }
                contract
            })
            .collect::<Vec<TradeOptionContract>>();

        contracts.truncate(filter.limit.unwrap_or(100) as usize);
        ListResponse {
            option_contracts: contracts,
            next_page_token: None,
        }
    }

    #[must_use]
    pub fn get_option_contract(&self, symbol_or_id: &str) -> Option<TradeOptionContract> {
        options_contracts::catalog().into_iter().find(|contract| {
            contract.id == symbol_or_id || contract.symbol.eq_ignore_ascii_case(symbol_or_id)
        })
    }

    #[must_use]
    pub fn legacy_calendar(
        &self,
        start: Option<chrono::NaiveDate>,
        end: Option<chrono::NaiveDate>,
        date_type: DateType,
    ) -> Vec<Calendar> {
        calendar::legacy_catalog()
            .into_iter()
            .filter(|day| {
                let selected_date = match date_type {
                    DateType::Trading => day.date.as_str(),
                    DateType::Settlement => day.settlement_date.as_str(),
                };
                chrono::NaiveDate::parse_from_str(selected_date, "%Y-%m-%d")
                    .ok()
                    .is_some_and(|date| {
                        start.is_none_or(|start| date >= start) && end.is_none_or(|end| date <= end)
                    })
            })
            .collect()
    }

    #[must_use]
    pub fn calendar_v3(
        &self,
        market: Market,
        start: Option<chrono::NaiveDate>,
        end: Option<chrono::NaiveDate>,
        timezone: Option<CalendarTimezone>,
    ) -> CalendarV3Response {
        calendar::v3_calendar(market, start, end, timezone)
    }

    #[must_use]
    pub fn legacy_clock(&self) -> Clock {
        clock::legacy_clock()
    }

    #[must_use]
    pub fn clock_v3(&self, markets: Vec<Market>, time: Option<String>) -> ClockV3Response {
        clock::clock_v3(markets, time)
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
        validate_limit_price_for_order_class(&current.order.order_class, replacement_limit_price)?;
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
        record_post_replace_effects(account, &replacement, &request_side, &market_quotes);

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

        let mut projected = open_positions
            .into_iter()
            .map(|position| {
                public_position_from_projection(positions::project_position_without_market(
                    &position,
                ))
            })
            .collect::<Vec<_>>();
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
        let snapshot = if let Some(snapshot) = position.market_snapshot.clone() {
            snapshot
        } else {
            self.instrument_snapshot(&position.instrument_identity.symbol)
                .await?
        };

        Ok(public_position_from_projection(project_position(
            &position, &snapshot,
        )))
    }

    pub async fn close_position(
        &self,
        api_key: &str,
        symbol_or_asset_id: &str,
        input: ClosePositionInput,
    ) -> Result<Order, MockStateError> {
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
        let snapshot = if let Some(snapshot) = position.market_snapshot.clone() {
            snapshot
        } else {
            self.instrument_snapshot(&position.instrument_identity.symbol)
                .await?
        };
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
        let market_quotes = HashMap::from([(
            position.instrument_identity.symbol.clone(),
            snapshot.clone(),
        )]);
        apply_fill_effects(account, &order, &request_side, Some(&market_quotes));

        Ok(order)
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
        ensure_do_not_exercise_eligible_position(&position)?;
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
    ) -> Result<ExerciseDetails, MockStateError> {
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
            position.market_snapshot.clone(),
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
            Some(InstrumentSnapshot::equity(
                parsed.strike_price,
                parsed.strike_price,
            )),
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

        Ok(ExerciseDetails {
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
            .market_data_overrides
            .write()
            .expect("market data overrides lock should not poison")
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
            .market_data_overrides
            .read()
            .expect("market data overrides lock should not poison")
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
            account_configurations: account::default_account_configurations(),
            orders: HashMap::new(),
            client_order_ids: HashMap::new(),
            executions: Vec::new(),
            positions: PositionBook::default(),
            activities: Vec::new(),
            watchlists: WatchlistBook::default(),
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

fn validate_limit_price_for_order_class(
    order_class: &OrderClass,
    limit_price: Option<Decimal>,
) -> Result<(), MockStateError> {
    if *order_class != OrderClass::Mleg
        && limit_price.is_some_and(|limit_price| limit_price <= Decimal::ZERO)
    {
        return Err(MockStateError::Conflict(
            "simple limit_price must be greater than 0".to_owned(),
        ));
    }

    Ok(())
}

fn resolve_close_qty(
    position: &positions::InstrumentPosition,
    input: &ClosePositionInput,
) -> Result<Decimal, MockStateError> {
    if input.qty.is_some() && input.percentage.is_some() {
        return Err(MockStateError::Conflict(
            "qty and percentage are mutually exclusive".to_owned(),
        ));
    }

    let available = position.net_qty.abs();
    let qty = if let Some(qty) = input.qty {
        validate_close_amount("qty", qty, None)?;
        qty
    } else if let Some(percentage) = input.percentage {
        validate_close_amount("percentage", percentage, Some(Decimal::new(100, 0)))?;
        (available * percentage / Decimal::new(100, 0)).round_dp(9)
    } else {
        available
    };

    if qty > available {
        return Err(MockStateError::Conflict(format!(
            "close quantity {qty} exceeds available position quantity {available}"
        )));
    }

    Ok(qty)
}

fn validate_close_amount(
    name: &str,
    value: Decimal,
    maximum: Option<Decimal>,
) -> Result<(), MockStateError> {
    if value <= Decimal::ZERO {
        return Err(MockStateError::Conflict(format!(
            "{name} must be greater than 0"
        )));
    }
    if maximum.is_some_and(|maximum| value > maximum) {
        return Err(MockStateError::Conflict(format!(
            "{name} must be at most 100"
        )));
    }
    if value.scale() > 9 {
        return Err(MockStateError::Conflict(format!(
            "{name} must have at most 9 decimal places"
        )));
    }

    Ok(())
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
        OrderType::Limit => order
            .limit_price
            .filter(|limit_price| *limit_price >= mid)
            .map(|_| mid),
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
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) {
    if order.status == OrderStatus::Filled {
        apply_fill_effects(account, order, request_side, Some(market_quotes));
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
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) {
    if order.status == OrderStatus::Filled {
        apply_fill_effects(account, order, request_side, Some(market_quotes));
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

fn apply_fill_effects(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
    market_quotes: Option<&HashMap<String, InstrumentSnapshot>>,
) {
    let price = order
        .filled_avg_price
        .expect("filled mock order should always have filled_avg_price");
    let qty = order.filled_qty;
    let cash_delta = if order.order_class == OrderClass::Mleg {
        (-price * qty).round_dp(8)
    } else {
        signed_cash_delta(request_side, qty, price)
    };
    account.cash_ledger.apply_delta(cash_delta);
    let occurred_at = order
        .filled_at
        .clone()
        .unwrap_or_else(|| order.updated_at.clone());
    let executions =
        execution_facts_from_order(account, order, request_side, market_quotes, &occurred_at);
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
    market_quotes: Option<&HashMap<String, InstrumentSnapshot>>,
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
                            market_quotes.and_then(|quotes| quotes.get(&leg.symbol).cloned()),
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
        market_quotes.and_then(|quotes| quotes.get(&order.symbol).cloned()),
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

fn matches_status_filter(order: &Order, status: Option<QueryOrderStatus>) -> bool {
    match status.unwrap_or(QueryOrderStatus::Open) {
        QueryOrderStatus::All => true,
        QueryOrderStatus::Open => !is_terminal_status(&order.status),
        QueryOrderStatus::Closed => is_terminal_status(&order.status),
    }
}

fn order_for_list(order: &Order, nested: bool) -> Order {
    if nested {
        return order.clone();
    }

    let mut projected = order.clone();
    projected.legs = None;
    projected
}

fn matches_order_asset_classes(order: &Order, asset_classes: Option<&[OrderAssetClass]>) -> bool {
    asset_classes.is_none_or(|asset_classes| {
        asset_classes.is_empty()
            || asset_classes.contains(&OrderAssetClass::All)
            || asset_classes
                .iter()
                .any(|asset_class| asset_class.to_string() == order.asset_class)
    })
}

fn matches_order_symbols(
    order: &Order,
    symbols: Option<&HashSet<String>>,
    asset_classes: Option<&[OrderAssetClass]>,
) -> bool {
    symbols.is_none_or(|symbols| {
        symbols.contains(&order.symbol.to_ascii_uppercase())
            || (asset_classes
                .is_some_and(|asset_classes| asset_classes.contains(&OrderAssetClass::UsOption))
                && positions::parse_option_symbol(&order.symbol).is_some_and(|contract| {
                    symbols.contains(&contract.underlying_symbol.to_ascii_uppercase())
                }))
    })
}

fn compare_order_submission(left: &Order, right: &Order) -> std::cmp::Ordering {
    compare_timestamp(&left.submitted_at, &right.submitted_at)
        .then_with(|| mock_order_sequence(&left.id).cmp(&mock_order_sequence(&right.id)))
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_timestamp(left: &str, right: &str) -> std::cmp::Ordering {
    match (
        chrono::DateTime::parse_from_rfc3339(left),
        chrono::DateTime::parse_from_rfc3339(right),
    ) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn mock_order_sequence(order_id: &str) -> Option<u64> {
    order_id.rsplit('-').next()?.parse().ok()
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
    _request_side: &OrderSide,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<Decimal> {
    let (best, worst) = mleg_best_worst_from_order(order, market_quotes)?;
    Some(((best + worst) / Decimal::from(2)).round_dp(2))
}

fn mleg_best_worst_from_order(
    order: &Order,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<(Decimal, Decimal)> {
    let quoted_legs = order
        .legs
        .as_ref()?
        .iter()
        .map(|leg| {
            quoted_leg_from_market(
                &leg.symbol,
                &leg.side,
                leg.ratio_qty.unwrap_or(1),
                market_quotes,
            )
        })
        .collect::<Option<Vec<_>>>()?;

    mleg_best_worst_from_quoted_legs(&quoted_legs)
}

fn infer_request_side(
    request_side: Option<OrderSide>,
    limit_price: Option<Decimal>,
    request_legs: Option<&[OptionLegRequest]>,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> OrderSide {
    if let Some(side) = request_side
        && side != OrderSide::Unspecified
    {
        return side;
    }

    if let Some(limit_price) = limit_price {
        if limit_price > Decimal::ZERO {
            return OrderSide::Buy;
        }
        if limit_price < Decimal::ZERO {
            return OrderSide::Sell;
        }
    }

    let Some(legs) = request_legs else {
        return OrderSide::Buy;
    };
    let Some(total) = mleg_raw_total_from_legs(legs, market_quotes) else {
        return OrderSide::Buy;
    };

    if total > Decimal::ZERO {
        OrderSide::Buy
    } else if total < Decimal::ZERO {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    }
}

fn mleg_raw_total_from_legs(
    legs: &[OptionLegRequest],
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<Decimal> {
    let quoted_legs = legs
        .iter()
        .map(|leg| {
            quoted_leg_from_market(
                &leg.symbol,
                leg.side.as_ref()?,
                leg.ratio_qty,
                market_quotes,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let (best, worst) = mleg_best_worst_from_quoted_legs(&quoted_legs)?;
    Some(((best + worst) / Decimal::from(2)).round_dp(2))
}

fn mleg_best_worst_from_quoted_legs(quoted_legs: &[QuotedLeg]) -> Option<(Decimal, Decimal)> {
    let range = execution_quote::best_worst(quoted_legs, Some(1)).ok()?;
    Some((
        Decimal::from_f64(range.per_structure.best_price)?,
        Decimal::from_f64(range.per_structure.worst_price)?,
    ))
}

fn quoted_leg_from_market(
    symbol: &str,
    side: &OrderSide,
    ratio_qty: u32,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<QuotedLeg> {
    let instrument = market_quotes.get(symbol)?;
    Some(QuotedLeg {
        contract: OptionContract {
            occ_symbol: symbol.trim().to_ascii_uppercase(),
            ..OptionContract::default()
        },
        order_side: match side {
            OrderSide::Buy => QuoteOrderSide::Buy,
            OrderSide::Sell => QuoteOrderSide::Sell,
            OrderSide::Unspecified => return None,
        },
        ratio_quantity: ratio_qty,
        quote: OptionQuote {
            bid: instrument.bid.to_f64().filter(|value| value.is_finite()),
            ask: instrument.ask.to_f64().filter(|value| value.is_finite()),
            mark: None,
            last: None,
        },
        snapshot: None,
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

fn option_leg_requests_from_orders(legs: &[OrderLeg]) -> Vec<OptionLegRequest> {
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
    previous_legs: Option<&[OrderLeg]>,
) -> Result<Option<Vec<OrderLeg>>, MockStateError> {
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
    previous_legs: Option<&[OrderLeg]>,
) -> Result<Vec<OrderLeg>, MockStateError> {
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
    previous_leg: Option<&OrderLeg>,
) -> OrderLeg {
    OrderLeg {
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
    previous_legs: Option<&[OrderLeg]>,
) -> Vec<OrderLeg> {
    legs.iter()
        .enumerate()
        .map(|(index, leg)| {
            let previous_leg = previous_legs.and_then(|legs| legs.get(index));
            let leg_qty = parent_qty * Decimal::from(leg.ratio_qty);
            OrderLeg {
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
        exchange: match projected.exchange.as_str() {
            "AMEX" => PositionExchange::Amex,
            "ARCA" => PositionExchange::Arca,
            "BATS" => PositionExchange::Bats,
            "NYSE" => PositionExchange::Nyse,
            "NASDAQ" => PositionExchange::Nasdaq,
            "NYSEARCA" => PositionExchange::NyseArca,
            "OTC" => PositionExchange::Otc,
            "CRYPTO" => PositionExchange::Crypto,
            _ => PositionExchange::Unspecified,
        },
        asset_class: match projected.asset_class.as_str() {
            "us_option" => AssetClass::UsOption,
            "crypto" => AssetClass::Crypto,
            "crypto_perp" => AssetClass::CryptoPerp,
            "treasury" => AssetClass::Treasury,
            "corporate" => AssetClass::Corporate,
            "global_equity" => AssetClass::GlobalEquity,
            "us_index" => AssetClass::UsIndex,
            "us_equity_chain" => AssetClass::UsEquityChain,
            "ipo" => AssetClass::Ipo,
            _ => AssetClass::UsEquity,
        },
        asset_marginable: projected.asset_marginable,
        side: if projected.side == "short" {
            TradePositionSide::Short
        } else {
            TradePositionSide::Long
        },
        qty: projected.qty,
        avg_entry_price: projected.avg_entry_price,
        market_value: projected.market_value,
        cost_basis: projected.cost_basis,
        unrealized_pl: projected.unrealized_pl,
        unrealized_plpc: projected.unrealized_plpc,
        unrealized_intraday_pl: projected.unrealized_intraday_pl,
        unrealized_intraday_plpc: projected.unrealized_intraday_plpc,
        current_price: projected.current_price,
        lastday_price: projected.lastday_price,
        change_today: projected.change_today,
        qty_available: Some(projected.qty_available),
        avg_entry_swap_rate: None,
        prev_swap_rate: None,
        swap_rate: None,
        usd: None,
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

fn ensure_do_not_exercise_eligible_position(
    position: &positions::InstrumentPosition,
) -> Result<(), MockStateError> {
    ensure_exercisable_long_option_position(position)?;
    let parsed = parse_option_symbol(&position.instrument_identity.symbol).ok_or_else(|| {
        MockStateError::Conflict(format!(
            "option symbol {} is not a parseable OCC contract",
            position.instrument_identity.symbol
        ))
    })?;
    let today = Utc::now().with_timezone(&New_York).date_naive();
    if parsed.expiration_date != today {
        return Err(MockStateError::Forbidden(
            "dne requests are only accepted on the expiration day of the option contract"
                .to_owned(),
        ));
    }

    Ok(())
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
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
        | TimeInForce::Fok
        | TimeInForce::Unspecified => None,
    }
}
