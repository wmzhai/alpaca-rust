use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use alpaca_core::QueryWriter;

use crate::Error;

use super::{
    OrderClass, OrderSide, OrderType, PositionIntent, QueryOrderStatus, SortDirection, StopLoss,
    TakeProfit, TimeInForce,
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
    pub asset_class: Option<String>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt("status", self.status);
        query.push_opt("limit", validate_limit(self.limit, 1, 500)?);
        query.push_opt("after", validate_optional_text("after", self.after)?);
        query.push_opt("until", validate_optional_text("until", self.until)?);
        query.push_opt("direction", self.direction);
        query.push_opt("nested", self.nested);
        if let Some(symbols) = validate_optional_symbols(self.symbols)? {
            query.push_csv("symbols", symbols);
        }
        query.push_opt("side", self.side);
        query.push_opt(
            "asset_class",
            validate_optional_text("asset_class", self.asset_class)?,
        );
        Ok(query.finish())
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
}

impl CreateRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Some(symbol) = &self.symbol {
            validate_required_text("symbol", symbol)?;
        }
        if let Some(client_order_id) = &self.client_order_id {
            validate_required_text("client_order_id", client_order_id)?;
        }
        if let Some(legs) = &self.legs {
            for leg in legs {
                leg.validate()?;
            }
        }
        if self.order_class == Some(OrderClass::Mleg) {
            validate_mleg_legs(self.legs.as_deref())?;
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
}

impl ReplaceRequest {
    pub(crate) fn into_json(self) -> Result<serde_json::Value, Error> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| Error::InvalidRequest(error.to_string()))
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Some(client_order_id) = &self.client_order_id {
            validate_required_text("client_order_id", client_order_id)?;
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

impl fmt::Display for OrderSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Unspecified => "",
        })
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use serde_json::json;

    use super::{CreateRequest, ReplaceRequest};
    use crate::orders::{OrderClass, OrderSide, OrderType, StopLoss, TakeProfit, TimeInForce};

    #[test]
    fn create_request_into_json_rounds_price_fields_to_two_decimals() {
        let request = CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::ONE),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1239, 3)),
            stop_price: Some(Decimal::new(2345, 3)),
            trail_price: Some(Decimal::new(3456, 3)),
            trail_percent: Some(Decimal::new(3456, 3)),
            order_class: Some(OrderClass::Bracket),
            take_profit: Some(TakeProfit {
                limit_price: Decimal::new(4567, 3),
            }),
            stop_loss: Some(StopLoss {
                stop_price: Decimal::new(5678, 3),
                limit_price: Some(Decimal::new(6789, 3)),
            }),
            ..CreateRequest::default()
        };

        let json = request.into_json().expect("request should serialize");

        assert_eq!(
            json,
            json!({
                "symbol": "SPY",
                "qty": "1",
                "side": "buy",
                "type": "limit",
                "time_in_force": "day",
                "limit_price": "1.24",
                "stop_price": "2.34",
                "trail_price": "3.46",
                "trail_percent": "3.456",
                "order_class": "bracket",
                "take_profit": {
                    "limit_price": "4.57",
                },
                "stop_loss": {
                    "stop_price": "5.68",
                    "limit_price": "6.79",
                }
            })
        );
    }

    #[test]
    fn replace_request_into_json_rounds_price_fields_to_two_decimals() {
        let request = ReplaceRequest {
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(Decimal::new(1239, 3)),
            stop_price: Some(Decimal::new(2345, 3)),
            trail: Some(Decimal::new(3456, 3)),
            ..ReplaceRequest::default()
        };

        let json = request.into_json().expect("request should serialize");

        assert_eq!(
            json,
            json!({
                "time_in_force": "day",
                "limit_price": "1.24",
                "stop_price": "2.34",
                "trail": "3.46"
            })
        );
    }
}
