use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountConfigurations {
    pub trade_confirm_email: Option<String>,
    pub suspend_trade: Option<bool>,
    /// Paper response extension observed outside the canonical OpenAPI schema.
    pub closing_transactions_only: Option<bool>,
    pub no_shorting: Option<bool>,
    pub fractional_trading: Option<bool>,
    pub max_margin_multiplier: Option<String>,
    pub max_options_trading_level: Option<u32>,
    pub ptp_no_exception_entry: Option<bool>,
    pub disable_overnight_trading: Option<bool>,
}
