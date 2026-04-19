use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::Error;

use super::{OrderStatus, OrderTerminalState, SubmitOrderStyle};

#[derive(Debug, Clone, Serialize, PartialEq, Default, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Execution {
    #[default]
    Market,
    Limit {
        #[ts(type = "string")]
        limit_price: Decimal,
    },
    DynamicLimit {
        #[ts(type = "string")]
        limit_price: Decimal,
        #[ts(type = "string")]
        start_price: Decimal,
        #[ts(type = "string")]
        end_price: Decimal,
        current_percentage: f64,
        percentage_step: f64,
        #[ts(type = "number")]
        interval_seconds: i64,
        last_adjustment_time: Option<NaiveDateTime>,
    },
    DynamicMarket {
        #[ts(type = "string")]
        #[serde(default)]
        limit_price: Decimal,
        #[serde(default)]
        start_percentage: f64,
        #[serde(default)]
        current_percentage: f64,
        #[serde(default)]
        percentage_step: f64,
        #[ts(type = "number")]
        #[serde(default)]
        interval_seconds: i64,
        #[serde(default)]
        last_adjustment_time: Option<NaiveDateTime>,
    },
}

impl<'de> Deserialize<'de> for Execution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let value = Value::deserialize(deserializer)?;

        if let Some(kind) = value.as_str() {
            return match kind {
                "market" => Ok(Self::Market),
                other => Err(D::Error::custom(format!(
                    "unsupported execution string: {other}"
                ))),
            };
        }

        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("execution must be an object like {\"type\": ...}"))?;

        let kind = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| D::Error::custom("execution.type is required"))?;

        let decimal_required = |field: &str| -> Result<Decimal, D::Error> {
            let raw = object
                .get(field)
                .cloned()
                .ok_or_else(|| D::Error::custom(format!("execution.{field} is required")))?;
            serde_json::from_value(raw)
                .map_err(|error| D::Error::custom(format!("execution.{field}: {error}")))
        };

        let decimal_default = |field: &str, default: Decimal| -> Result<Decimal, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(default),
            }
        };

        let f64_required = |field: &str| -> Result<f64, D::Error> {
            let raw = object
                .get(field)
                .cloned()
                .ok_or_else(|| D::Error::custom(format!("execution.{field} is required")))?;
            serde_json::from_value(raw)
                .map_err(|error| D::Error::custom(format!("execution.{field}: {error}")))
        };

        let f64_default = |field: &str, default: f64| -> Result<f64, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(default),
            }
        };

        let i64_required = |field: &str| -> Result<i64, D::Error> {
            let raw = object
                .get(field)
                .cloned()
                .ok_or_else(|| D::Error::custom(format!("execution.{field} is required")))?;
            serde_json::from_value(raw)
                .map_err(|error| D::Error::custom(format!("execution.{field}: {error}")))
        };

        let i64_default = |field: &str, default: i64| -> Result<i64, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(default),
            }
        };

        let datetime_default = |field: &str| -> Result<Option<NaiveDateTime>, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(None),
            }
        };

        match kind {
            "market" => Ok(Self::Market),
            "limit" => Ok(Self::Limit {
                limit_price: decimal_required("limit_price")?,
            }),
            "dynamic_limit" => Ok(Self::DynamicLimit {
                limit_price: decimal_required("limit_price")?,
                start_price: decimal_required("start_price")?,
                end_price: decimal_required("end_price")?,
                current_percentage: f64_required("current_percentage")?,
                percentage_step: f64_required("percentage_step")?,
                interval_seconds: i64_required("interval_seconds")?,
                last_adjustment_time: datetime_default("last_adjustment_time")?,
            }),
            "dynamic_market" => Ok(Self::DynamicMarket {
                limit_price: decimal_default("limit_price", Decimal::ZERO)?,
                start_percentage: f64_default("start_percentage", 0.0)?,
                current_percentage: f64_default("current_percentage", 0.0)?,
                percentage_step: f64_default("percentage_step", 0.0)?,
                interval_seconds: i64_default("interval_seconds", 0)?,
                last_adjustment_time: datetime_default("last_adjustment_time")?,
            }),
            other => Err(D::Error::custom(format!(
                "unsupported execution.type: {other}"
            ))),
        }
    }
}

impl Execution {
    fn progress_decimal(value: f64) -> Result<Decimal, Error> {
        Decimal::try_from(value)
            .map_err(|_| Error::InvalidRequest(format!("invalid execution progress: {value}")))
    }

    #[must_use]
    pub fn dynamic_progress(&self) -> Option<(f64, &'static str)> {
        match self {
            Self::DynamicLimit {
                current_percentage, ..
            } => Some((*current_percentage, "DynamicLimit")),
            Self::DynamicMarket {
                current_percentage, ..
            } => Some((*current_percentage, "DynamicMarket")),
            _ => None,
        }
    }

    #[must_use]
    pub fn progress_and_price(&self) -> (f64, String) {
        match self {
            Self::DynamicLimit {
                current_percentage,
                limit_price,
                ..
            }
            | Self::DynamicMarket {
                current_percentage,
                limit_price,
                ..
            } => (*current_percentage, limit_price.to_string()),
            Self::Limit { limit_price } => (1.0, limit_price.to_string()),
            Self::Market => (1.0, "MARKET".to_string()),
        }
    }

    #[must_use]
    pub fn limit_price(&self) -> Option<Decimal> {
        match self {
            Self::Market => None,
            Self::Limit { limit_price }
            | Self::DynamicLimit { limit_price, .. }
            | Self::DynamicMarket { limit_price, .. } => Some(*limit_price),
        }
    }

    pub fn advance_dynamic_limit(&self, now: NaiveDateTime) -> Result<Self, Error> {
        match self {
            Self::DynamicLimit {
                limit_price,
                start_price,
                end_price,
                current_percentage,
                percentage_step,
                interval_seconds,
                last_adjustment_time,
            } => {
                if *current_percentage >= 1.0 {
                    return Ok(self.clone());
                }

                if let Some(last_time) = last_adjustment_time {
                    let elapsed = now.signed_duration_since(*last_time).num_seconds();
                    if elapsed < *interval_seconds {
                        return Ok(self.clone());
                    }
                }

                let next_percentage = (*current_percentage + *percentage_step).min(1.0);
                if next_percentage >= 1.0 {
                    return Ok(Self::Limit {
                        limit_price: *end_price,
                    });
                }

                let next_limit_price = (*start_price
                    + (*end_price - *start_price) * Self::progress_decimal(next_percentage)?)
                .round_dp(2);

                Ok(Self::DynamicLimit {
                    limit_price: if next_limit_price.is_zero() {
                        *limit_price
                    } else {
                        next_limit_price
                    },
                    start_price: *start_price,
                    end_price: *end_price,
                    current_percentage: next_percentage,
                    percentage_step: *percentage_step,
                    interval_seconds: *interval_seconds,
                    last_adjustment_time: Some(now),
                })
            }
            _ => Err(Error::InvalidRequest(
                "advance_dynamic_limit() only supports dynamic_limit execution".to_string(),
            )),
        }
    }

    pub fn advance_dynamic_market(
        &self,
        best: Decimal,
        worst: Decimal,
        now: NaiveDateTime,
    ) -> Result<Self, Error> {
        match self {
            Self::DynamicMarket {
                start_percentage,
                current_percentage,
                percentage_step,
                interval_seconds,
                last_adjustment_time,
                ..
            } => {
                if *current_percentage >= 1.0 {
                    return Ok(Self::Market);
                }

                if let Some(last_time) = last_adjustment_time {
                    let elapsed = now.signed_duration_since(*last_time).num_seconds();
                    if elapsed < *interval_seconds {
                        return Ok(self.clone());
                    }
                }

                let next_percentage = (*current_percentage + *percentage_step).min(1.0);
                if next_percentage >= 1.0 {
                    return Ok(Self::Market);
                }

                let next_limit_price =
                    best + (worst - best) * Self::progress_decimal(next_percentage)?;

                Ok(Self::DynamicMarket {
                    limit_price: next_limit_price,
                    start_percentage: *start_percentage,
                    current_percentage: next_percentage,
                    percentage_step: *percentage_step,
                    interval_seconds: *interval_seconds,
                    last_adjustment_time: Some(now),
                })
            }
            _ => Err(Error::InvalidRequest(
                "advance_dynamic_market() only supports dynamic_market execution".to_string(),
            )),
        }
    }

    #[must_use]
    pub fn submit_order_style(&self) -> SubmitOrderStyle {
        match self {
            Self::Market => SubmitOrderStyle::Market,
            Self::Limit { limit_price }
            | Self::DynamicLimit { limit_price, .. }
            | Self::DynamicMarket { limit_price, .. } => SubmitOrderStyle::Limit {
                limit_price: limit_price.round_dp(2),
            },
        }
    }

    #[must_use]
    pub fn needs_market_retry(&self, status: &str) -> bool {
        if let Self::DynamicMarket {
            current_percentage, ..
        } = self
        {
            *current_percentage >= 1.0
                && matches!(
                    OrderStatus::parse(status)
                        .ok()
                        .and_then(OrderStatus::terminal_state),
                    Some(OrderTerminalState::Canceled) | Some(OrderTerminalState::Rejected)
                )
        } else {
            false
        }
    }

    pub fn from_order_type(order_type: &str, limit_price: Option<Decimal>) -> Result<Self, Error> {
        match order_type.trim() {
            "market" => Ok(Self::Market),
            "limit" => Ok(Self::Limit {
                limit_price: limit_price.ok_or_else(|| {
                    Error::InvalidRequest("limit order is missing limit_price".to_string())
                })?,
            }),
            _ => Ok(Self::Market),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use ts_rs::TS;

    use super::Execution;
    use crate::orders::SubmitOrderStyle;

    fn dt(hour: u32, minute: u32, second: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 4, 18)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    #[test]
    fn deserializes_market_and_dynamic_variants() {
        let market: Execution = serde_json::from_str("\"market\"").unwrap();
        assert_eq!(market, Execution::Market);

        let dynamic_limit: Execution = serde_json::from_value(serde_json::json!({
            "type": "dynamic_limit",
            "limit_price": "1.20",
            "start_price": "1.20",
            "end_price": "1.80",
            "current_percentage": 0.25,
            "percentage_step": 0.25,
            "interval_seconds": 15,
            "last_adjustment_time": "2026-04-18T10:00:00"
        }))
        .unwrap();

        assert_eq!(
            dynamic_limit,
            Execution::DynamicLimit {
                limit_price: Decimal::new(120, 2),
                start_price: Decimal::new(120, 2),
                end_price: Decimal::new(180, 2),
                current_percentage: 0.25,
                percentage_step: 0.25,
                interval_seconds: 15,
                last_adjustment_time: Some(dt(10, 0, 0)),
            }
        );
    }

    #[test]
    fn advances_dynamic_limit_and_rounds_limit_price() {
        let execution = Execution::DynamicLimit {
            limit_price: Decimal::new(120, 2),
            start_price: Decimal::new(120, 2),
            end_price: Decimal::new(180, 2),
            current_percentage: 0.0,
            percentage_step: 0.3333,
            interval_seconds: 0,
            last_adjustment_time: None,
        };

        let advanced = execution.advance_dynamic_limit(dt(10, 5, 0)).unwrap();
        assert_eq!(
            advanced,
            Execution::DynamicLimit {
                limit_price: Decimal::new(140, 2),
                start_price: Decimal::new(120, 2),
                end_price: Decimal::new(180, 2),
                current_percentage: 0.3333,
                percentage_step: 0.3333,
                interval_seconds: 0,
                last_adjustment_time: Some(dt(10, 5, 0)),
            }
        );
    }

    #[test]
    fn advances_dynamic_market_until_market() {
        let execution = Execution::DynamicMarket {
            limit_price: Decimal::new(100, 2),
            start_percentage: 0.4,
            current_percentage: 0.4,
            percentage_step: 0.3,
            interval_seconds: 0,
            last_adjustment_time: None,
        };

        let advanced = execution
            .advance_dynamic_market(
                Decimal::new(-2000, 2),
                Decimal::new(-1000, 2),
                dt(10, 10, 0),
            )
            .unwrap();
        assert_eq!(
            advanced,
            Execution::DynamicMarket {
                limit_price: Decimal::new(-1300, 2),
                start_percentage: 0.4,
                current_percentage: 0.7,
                percentage_step: 0.3,
                interval_seconds: 0,
                last_adjustment_time: Some(dt(10, 10, 0)),
            }
        );

        let terminal = advanced
            .advance_dynamic_market(
                Decimal::new(-2000, 2),
                Decimal::new(-1000, 2),
                dt(10, 11, 0),
            )
            .unwrap();
        assert_eq!(terminal, Execution::Market);
    }

    #[test]
    fn exposes_submit_style_limit_price_and_retry_logic() {
        let limit = Execution::Limit {
            limit_price: Decimal::new(1234, 3),
        };
        assert_eq!(limit.limit_price(), Some(Decimal::new(1234, 3)));
        assert_eq!(
            limit.submit_order_style(),
            SubmitOrderStyle::Limit {
                limit_price: Decimal::new(123, 2),
            }
        );

        let dynamic_market = Execution::DynamicMarket {
            limit_price: Decimal::new(-1879, 2),
            start_percentage: 0.55,
            current_percentage: 1.0,
            percentage_step: 0.15,
            interval_seconds: 5,
            last_adjustment_time: Some(dt(10, 12, 0)),
        };
        assert!(dynamic_market.needs_market_retry("canceled"));
        assert!(!dynamic_market.needs_market_retry("filled"));
    }

    #[test]
    fn builds_from_order_type_and_exports_ts_shape() {
        assert_eq!(
            Execution::from_order_type("limit", Some(Decimal::new(125, 2))).unwrap(),
            Execution::Limit {
                limit_price: Decimal::new(125, 2),
            }
        );
        assert_eq!(
            Execution::from_order_type("market", None).unwrap(),
            Execution::Market
        );
        assert!(Execution::from_order_type("limit", None).is_err());

        let export = Execution::export_to_string().unwrap();
        assert!(export.contains("type Execution"));
        assert!(export.contains("\"dynamic_market\""));
    }
}
