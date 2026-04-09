use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountConfigurations {
    pub dtbp_check: Option<String>,
    pub trade_confirm_email: Option<String>,
    pub suspend_trade: Option<bool>,
    pub no_shorting: Option<bool>,
    pub fractional_trading: Option<bool>,
    pub max_margin_multiplier: Option<String>,
    pub max_options_trading_level: Option<u32>,
    pub pdt_check: Option<String>,
    pub ptp_no_exception_entry: Option<bool>,
    pub disable_overnight_trading: Option<bool>,
}
