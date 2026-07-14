use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use alpaca_core::QueryWriter;

use crate::Error;

use super::{
    OrderAssetClass, OrderClass, OrderSide, OrderType, PositionIntent, QueryOrderStatus,
    SortDirection, StopLoss, TakeProfit, TimeInForce,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub status: Option<QueryOrderStatus>,
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub until: Option<String>,
    pub direction: Option<SortDirection>,
    pub nested: Option<bool>,
    pub symbols: Option<Vec<String>>,
    pub side: Option<OrderSide>,
    pub asset_class: Option<Vec<OrderAssetClass>>,
    pub before_order_id: Option<String>,
    pub after_order_id: Option<String>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let after = validate_optional_text("after", self.after)?;
        let until = validate_optional_text("until", self.until)?;
        let before_order_id = validate_optional_text("before_order_id", self.before_order_id)?;
        let after_order_id = validate_optional_text("after_order_id", self.after_order_id)?;
        if before_order_id.is_some() && after_order_id.is_some() {
            return Err(Error::InvalidRequest(
                "before_order_id and after_order_id are mutually exclusive".to_owned(),
            ));
        }
        if (before_order_id.is_some() || after_order_id.is_some())
            && (after.is_some() || until.is_some())
        {
            return Err(Error::InvalidRequest(
                "order ID cursors cannot be combined with after or until".to_owned(),
            ));
        }

        let mut query = QueryWriter::default();
        query.push_opt("status", self.status);
        query.push_opt("limit", validate_limit(self.limit, 1, 500)?);
        query.push_opt("after", after);
        query.push_opt("until", until);
        query.push_opt("direction", self.direction);
        query.push_opt("nested", self.nested);
        if let Some(symbols) = validate_optional_symbols(self.symbols)? {
            query.push_csv("symbols", symbols);
        }
        query.push_opt("side", self.side);
        if let Some(asset_classes) = self.asset_class {
            query.push_csv("asset_class", asset_classes);
        }
        query.push_opt("before_order_id", before_order_id);
        query.push_opt("after_order_id", after_order_id);
        Ok(query.finish())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GetRequest {
    pub nested: Option<bool>,
}

impl GetRequest {
    pub(crate) fn into_query(self) -> Vec<(String, String)> {
        let mut query = QueryWriter::default();
        query.push_opt("nested", self.nested);
        query.finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub qty: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub notional: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<OrderSide>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<OrderType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub limit_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub stop_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub trail_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub trail_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_hours: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_class: Option<OrderClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<TakeProfit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<StopLoss>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legs: Option<Vec<OptionLegRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_intent: Option<PositionIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_instructions: Option<AdvancedInstructions>,
}

impl CreateRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    pub fn validate(&self) -> Result<(), Error> {
        let order_type = self
            .r#type
            .ok_or_else(|| Error::InvalidRequest("type is required".to_owned()))?;
        let time_in_force = self
            .time_in_force
            .ok_or_else(|| Error::InvalidRequest("time_in_force is required".to_owned()))?;
        if order_type == OrderType::Unspecified {
            return Err(Error::InvalidRequest(
                "type must be a canonical request value".to_owned(),
            ));
        }
        if matches!(time_in_force, TimeInForce::Gtd | TimeInForce::Unspecified) {
            return Err(Error::InvalidRequest(
                "time_in_force must be a canonical request value".to_owned(),
            ));
        }

        if self.order_class == Some(OrderClass::Mleg) {
            validate_mleg_legs(self.legs.as_deref())?;
            if self.qty.is_none() {
                return Err(Error::InvalidRequest(
                    "qty is required when order_class is mleg".to_owned(),
                ));
            }
            if !matches!(order_type, OrderType::Market | OrderType::Limit)
                || time_in_force != TimeInForce::Day
            {
                return Err(Error::InvalidRequest(
                    "mleg orders require type=market|limit and time_in_force=day".to_owned(),
                ));
            }
        } else {
            validate_required_text(
                "symbol",
                self.symbol.as_deref().ok_or_else(|| {
                    Error::InvalidRequest(
                        "symbol is required unless order_class is mleg".to_owned(),
                    )
                })?,
            )?;
            if self.side.is_none_or(|side| side == OrderSide::Unspecified) {
                return Err(Error::InvalidRequest(
                    "side is required unless order_class is mleg".to_owned(),
                ));
            }
            if self.qty.is_none() && self.notional.is_none() {
                return Err(Error::InvalidRequest(
                    "one of qty or notional is required".to_owned(),
                ));
            }
        }
        if let Some(client_order_id) = &self.client_order_id {
            validate_required_text("client_order_id", client_order_id)?;
            if client_order_id.chars().count() > 128 {
                return Err(Error::InvalidRequest(
                    "client_order_id must contain at most 128 characters".to_owned(),
                ));
            }
        }
        if let Some(legs) = &self.legs {
            for leg in legs {
                leg.validate()?;
            }
        }
        if self.qty.is_some() && self.notional.is_some() {
            return Err(Error::InvalidRequest(
                "qty and notional are mutually exclusive".to_owned(),
            ));
        }
        if let Some(qty) = self.qty {
            validate_positive_decimal("qty", qty, 9)?;
            if !qty.fract().is_zero() && time_in_force != TimeInForce::Day {
                return Err(Error::InvalidRequest(
                    "fractional qty requires time_in_force=day".to_owned(),
                ));
            }
        }
        if let Some(notional) = self.notional {
            validate_positive_decimal("notional", notional, 9)?;
            if order_type != OrderType::Market
                || !matches!(time_in_force, TimeInForce::Day | TimeInForce::Gtc)
            {
                return Err(Error::InvalidRequest(
                    "notional requires type=market and time_in_force=day, or gtc for IPO indications"
                        .to_owned(),
                ));
            }
        }
        if matches!(order_type, OrderType::Limit | OrderType::StopLimit)
            && self.limit_price.is_none()
        {
            return Err(Error::InvalidRequest(
                "limit_price is required for limit and stop_limit orders".to_owned(),
            ));
        }
        if matches!(order_type, OrderType::Stop | OrderType::StopLimit) && self.stop_price.is_none()
        {
            return Err(Error::InvalidRequest(
                "stop_price is required for stop and stop_limit orders".to_owned(),
            ));
        }
        if order_type == OrderType::TrailingStop {
            if self.trail_price.is_some() == self.trail_percent.is_some() {
                return Err(Error::InvalidRequest(
                    "trailing_stop requires exactly one of trail_price or trail_percent".to_owned(),
                ));
            }
        }
        if self.extended_hours == Some(true)
            && (order_type != OrderType::Limit
                || !matches!(time_in_force, TimeInForce::Day | TimeInForce::Gtc))
        {
            return Err(Error::InvalidRequest(
                "extended_hours requires type=limit and time_in_force=day|gtc".to_owned(),
            ));
        }
        if let Some(instructions) = &self.advanced_instructions {
            instructions.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvancedAlgorithm {
    #[serde(rename = "DMA")]
    Dma,
    #[serde(rename = "TWAP")]
    Twap,
    #[serde(rename = "VWAP")]
    Vwap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvancedDestination {
    #[serde(rename = "NYSE")]
    Nyse,
    #[serde(rename = "NASDAQ")]
    Nasdaq,
    #[serde(rename = "ARCA")]
    Arca,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdvancedInstructions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<AdvancedAlgorithm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<AdvancedDestination>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub display_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub max_percentage: Option<Decimal>,
}

impl AdvancedInstructions {
    fn validate(&self) -> Result<(), Error> {
        if let Some(display_qty) = self.display_qty {
            if display_qty <= Decimal::ZERO
                || !display_qty.fract().is_zero()
                || display_qty % Decimal::new(100, 0) != Decimal::ZERO
            {
                return Err(Error::InvalidRequest(
                    "advanced_instructions.display_qty must use positive round-lot increments"
                        .to_owned(),
                ));
            }
        }
        if let Some(max_percentage) = self.max_percentage {
            if max_percentage <= Decimal::ZERO || max_percentage >= Decimal::ONE {
                return Err(Error::InvalidRequest(
                    "advanced_instructions.max_percentage must be greater than 0 and less than 1"
                        .to_owned(),
                ));
            }
            if max_percentage.scale() > 3 {
                return Err(Error::InvalidRequest(
                    "advanced_instructions.max_percentage must have at most 3 decimal places"
                        .to_owned(),
                ));
            }
        }

        let start = self
            .start_time
            .as_deref()
            .map(|value| validate_rfc3339("advanced_instructions.start_time", value))
            .transpose()?;
        let end = self
            .end_time
            .as_deref()
            .map(|value| validate_rfc3339("advanced_instructions.end_time", value))
            .transpose()?;
        if start.zip(end).is_some_and(|(start, end)| start >= end) {
            return Err(Error::InvalidRequest(
                "advanced_instructions.start_time must be before end_time".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ReplaceRequest {
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::string_contract::serialize_option_decimal"
    )]
    pub qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub limit_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub stop_price: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "alpaca_core::decimal::price_string_contract::serialize_option_decimal"
    )]
    pub trail: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_instructions: Option<AdvancedInstructions>,
}

impl ReplaceRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    pub fn validate(&self) -> Result<(), Error> {
        if let Some(qty) = self.qty {
            validate_positive_decimal("qty", qty, 9)?;
            if !qty.fract().is_zero() {
                return Err(Error::InvalidRequest(
                    "replacement qty must use full shares".to_owned(),
                ));
            }
        }
        if matches!(
            self.time_in_force,
            Some(TimeInForce::Gtd | TimeInForce::Unspecified)
        ) {
            return Err(Error::InvalidRequest(
                "time_in_force must be a canonical request value".to_owned(),
            ));
        }
        if let Some(client_order_id) = &self.client_order_id {
            validate_required_text("client_order_id", client_order_id)?;
            if client_order_id.chars().count() > 128 {
                return Err(Error::InvalidRequest(
                    "client_order_id must contain at most 128 characters".to_owned(),
                ));
            }
        }
        if let Some(instructions) = &self.advanced_instructions {
            instructions.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionLegRequest {
    pub symbol: String,
    #[serde(
        deserialize_with = "alpaca_core::integer::deserialize_u32_from_string_or_number",
        serialize_with = "alpaca_core::integer::string_contract::serialize_u32"
    )]
    pub ratio_qty: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<OrderSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_intent: Option<PositionIntent>,
}

impl OptionLegRequest {
    fn validate(&self) -> Result<(), Error> {
        validate_required_text("symbol", &self.symbol)?;
        if self.ratio_qty == 0 {
            return Err(Error::InvalidRequest(
                "ratio_qty must be greater than 0".to_owned(),
            ));
        }

        Ok(())
    }
}

pub(crate) fn validate_order_id(order_id: &str) -> Result<String, Error> {
    validate_required_path_segment("order_id", order_id)
}

pub(crate) fn validate_client_order_id(client_order_id: &str) -> Result<String, Error> {
    validate_required_path_segment("client_order_id", client_order_id)
}

fn validate_optional_text(
    name: &'static str,
    value: Option<String>,
) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
}

fn validate_optional_symbols(value: Option<Vec<String>>) -> Result<Option<Vec<String>>, Error> {
    match value {
        None => Ok(None),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(
            "symbols must contain at least one symbol".to_owned(),
        )),
        Some(values) => values
            .into_iter()
            .map(|value| validate_required_text("symbols", &value))
            .collect::<Result<Vec<_>, Error>>()
            .map(Some),
    }
}

fn validate_required_text(name: &str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{name} must not be empty or whitespace-only"
        )));
    }

    Ok(trimmed.to_owned())
}

fn validate_required_path_segment(name: &str, value: &str) -> Result<String, Error> {
    let value = validate_required_text(name, value)?;
    if value.contains('/') {
        return Err(Error::InvalidRequest(format!(
            "{name} must not contain `/`"
        )));
    }

    Ok(value)
}

fn validate_limit(limit: Option<u32>, min: u32, max: u32) -> Result<Option<u32>, Error> {
    match limit {
        Some(limit) if !(min..=max).contains(&limit) => Err(Error::InvalidRequest(format!(
            "limit must be between {min} and {max}"
        ))),
        _ => Ok(limit),
    }
}

fn validate_positive_decimal(name: &str, value: Decimal, max_scale: u32) -> Result<(), Error> {
    if value <= Decimal::ZERO {
        return Err(Error::InvalidRequest(format!(
            "{name} must be greater than 0"
        )));
    }
    if value.scale() > max_scale {
        return Err(Error::InvalidRequest(format!(
            "{name} must have at most {max_scale} decimal places"
        )));
    }

    Ok(())
}

fn validate_rfc3339(
    name: &str,
    value: &str,
) -> Result<chrono::DateTime<chrono::FixedOffset>, Error> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| Error::InvalidRequest(format!("{name} must use RFC3339")))
}

fn validate_mleg_legs(legs: Option<&[OptionLegRequest]>) -> Result<(), Error> {
    let legs = legs.ok_or_else(|| {
        Error::InvalidRequest(
            "legs must contain 2 to 4 option legs when order_class is mleg".to_owned(),
        )
    })?;

    if !(2..=4).contains(&legs.len()) {
        return Err(Error::InvalidRequest(
            "legs must contain 2 to 4 option legs when order_class is mleg".to_owned(),
        ));
    }

    let gcd = legs.iter().fold(0, |current, leg| {
        if current == 0 {
            leg.ratio_qty
        } else {
            greatest_common_divisor(current, leg.ratio_qty)
        }
    });

    if gcd != 1 {
        return Err(Error::InvalidRequest(
            "ratio_qty values across mleg legs must use the simplest whole-number ratio".to_owned(),
        ));
    }

    Ok(())
}

fn greatest_common_divisor(lhs: u32, rhs: u32) -> u32 {
    let mut lhs = lhs;
    let mut rhs = rhs;
    while rhs != 0 {
        let remainder = lhs % rhs;
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

impl fmt::Display for QueryOrderStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        })
    }
}

impl fmt::Display for SortDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        })
    }
}

impl fmt::Display for OrderAssetClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UsEquity => "us_equity",
            Self::UsOption => "us_option",
            Self::Crypto => "crypto",
            Self::CryptoPerp => "crypto_perp",
            Self::Treasury => "treasury",
            Self::Corporate => "corporate",
            Self::GlobalEquity => "global_equity",
            Self::UsIndex => "us_index",
            Self::UsEquityChain => "us_equity_chain",
            Self::Ipo => "ipo",
            Self::All => "all",
        })
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Unspecified => "",
        })
    }
}
