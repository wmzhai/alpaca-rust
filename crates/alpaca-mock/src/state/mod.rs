use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod account;
mod market_data;

use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

pub use market_data::{DEFAULT_STOCK_SYMBOL, InstrumentSnapshot, LiveMarketDataBridge};

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
        }
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
