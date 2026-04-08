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
        Self {
            id: api_key.to_owned(),
            account_number: format!("mock-account-{api_key}"),
            status: "ACTIVE".to_owned(),
            currency: "USD".to_owned(),
        }
    }
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
        account_number: profile.account_number.clone(),
        status: profile.status.clone(),
        currency: Some(profile.currency.clone()),
        cash: Some(cash),
        portfolio_value: Some(cash),
        non_marginable_buying_power: Some(large_buying_power),
        accrued_fees: Some(Decimal::ZERO),
        pending_transfer_in: Some(Decimal::ZERO),
        pending_transfer_out: Some(Decimal::ZERO),
        pattern_day_trader: Some(false),
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
        initial_margin: Some(Decimal::ZERO),
        maintenance_margin: Some(Decimal::ZERO),
        sma: Some(Decimal::ZERO),
        daytrade_count: Some(0),
        last_maintenance_margin: Some(Decimal::ZERO),
        daytrading_buying_power: Some(large_buying_power),
        regt_buying_power: Some(large_buying_power),
        options_buying_power: Some(large_buying_power),
        options_approved_level: Some(0),
        options_trading_level: Some(0),
        intraday_adjustments: Some(Decimal::ZERO),
        pending_reg_taf_fees: Some(Decimal::ZERO),
        ..alpaca_trade::account::Account::default()
    }
}
