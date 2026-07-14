use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rust_decimal::Decimal;

use crate::state::{VirtualAccountState, account_profile, cash_balance};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountProfile {
    pub id: String,
    pub account_number: String,
    pub status: String,
    pub currency: String,
}

impl AccountProfile {
    pub fn new(api_key: &str) -> Self {
        let id = stable_mock_uuid(api_key);
        Self {
            account_number: format!("MOCK-{}", id.replace('-', "").to_uppercase()),
            id,
            status: "ACTIVE".to_owned(),
            currency: "USD".to_owned(),
        }
    }
}

fn stable_mock_uuid(api_key: &str) -> String {
    let high = hash_account_key("alpaca-mock-account-high", api_key);
    let low = hash_account_key("alpaca-mock-account-low", api_key);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&high.to_be_bytes());
    bytes[8..].copy_from_slice(&low.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

fn hash_account_key(namespace: &str, api_key: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    namespace.hash(&mut hasher);
    api_key.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashLedger {
    cash: Decimal,
}

impl CashLedger {
    pub fn seeded_default() -> Self {
        Self {
            cash: Decimal::new(1_000_000, 0),
        }
    }

    pub fn cash_balance(&self) -> Decimal {
        self.cash
    }

    pub fn apply_delta(&mut self, delta: Decimal) {
        self.cash += delta;
    }
}

pub(crate) fn project_account(state: &VirtualAccountState) -> alpaca_trade::account::Account {
    let profile = account_profile(state);
    let cash = cash_balance(state);
    let large_buying_power = Decimal::new(9_999_999, 0);
    let multiplier = Decimal::new(4, 0);

    alpaca_trade::account::Account {
        id: profile.id.clone(),
        status: profile.status.clone(),
        account_number: Some(profile.account_number.clone()),
        currency: Some(profile.currency.clone()),
        crypto_status: Some("ACTIVE".to_owned()),
        crypto_tier: Some(1),
        cash: Some(cash),
        portfolio_value: Some(cash),
        non_marginable_buying_power: Some(large_buying_power),
        accrued_fees: Some(Decimal::ZERO),
        pending_transfer_in: Some(Decimal::ZERO),
        pending_transfer_out: Some(Decimal::ZERO),
        trade_suspended_by_user: Some(false),
        trading_blocked: Some(false),
        transfers_blocked: Some(false),
        account_blocked: Some(false),
        shorting_enabled: Some(true),
        long_market_value: Some(Decimal::ZERO),
        short_market_value: Some(Decimal::ZERO),
        equity: Some(cash),
        last_equity: Some(cash),
        multiplier: Some(multiplier),
        buying_power: Some(large_buying_power),
        effective_buying_power: Some(large_buying_power),
        initial_margin: Some(Decimal::ZERO),
        maintenance_margin: Some(Decimal::ZERO),
        sma: Some(Decimal::ZERO),
        last_maintenance_margin: Some(Decimal::ZERO),
        regt_buying_power: Some(large_buying_power),
        options_buying_power: Some(large_buying_power),
        options_approved_level: Some(0),
        options_trading_level: Some(0),
        intraday_adjustments: Some(Decimal::ZERO),
        pending_reg_taf_fees: Some(Decimal::ZERO),
        position_market_value: Some(Decimal::ZERO),
        admin_configurations: Some(serde_json::json!({})),
        user_configurations: Some(serde_json::json!({})),
        ..alpaca_trade::account::Account::default()
    }
}

pub(crate) fn default_account_configurations()
-> alpaca_trade::account_configurations::AccountConfigurations {
    alpaca_trade::account_configurations::AccountConfigurations {
        trade_confirm_email: Some("all".to_owned()),
        suspend_trade: Some(false),
        closing_transactions_only: Some(false),
        no_shorting: Some(false),
        fractional_trading: Some(true),
        max_margin_multiplier: Some("4".to_owned()),
        max_options_trading_level: None,
        ptp_no_exception_entry: Some(false),
        disable_overnight_trading: Some(false),
    }
}
