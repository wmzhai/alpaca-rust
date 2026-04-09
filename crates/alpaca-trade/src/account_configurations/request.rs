use serde::Serialize;

use crate::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct UpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dtbp_check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_confirm_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suspend_trade: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_shorting: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fractional_trading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_margin_multiplier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_options_trading_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdt_check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ptp_no_exception_entry: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_overnight_trading: Option<bool>,
}

impl UpdateRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        serde_json::to_value(self).map_err(|error| {
            Error::InvalidRequest(format!("invalid account configurations body: {error}"))
        })
    }
}
