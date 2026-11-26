use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::Error;

use super::dynamic_market::{
    install_submission_benchmark, step_dynamic_market_state, DynamicMarketKind,
    DynamicMarketProfile, DynamicMarketQuote, DynamicMarketState, DynamicMarketStepOptions,
};
use super::SubmitOrderStyle;

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicMarketStepAction {
    Wait(Execution),
    Replace(Execution),
    Recreate(Execution),
    MarkStalled(Execution),
    MissingQuote,
}

impl DynamicMarketStepAction {
    pub fn execution(&self) -> Option<&Execution> {
        match self {
            Self::Wait(execution)
            | Self::Replace(execution)
            | Self::Recreate(execution)
            | Self::MarkStalled(execution) => Some(execution),
            Self::MissingQuote => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Default, TS)]
#[ts(export_to = "../../../packages/alpaca-trade/src/generated/")]
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
        #[serde(default)]
        profile: DynamicMarketProfile,
        #[serde(default)]
        start_at_mid: bool,
        #[ts(type = "string | null")]
        #[serde(default)]
        initial_mid: Option<Decimal>,
        #[ts(type = "string | null")]
        #[serde(default)]
        initial_spread: Option<Decimal>,
        #[serde(default)]
        step_index: u16,
        #[serde(default)]
        step_entered_at: Option<NaiveDateTime>,
        #[serde(default)]
        round: u8,
        #[serde(default)]
        max_rounds: u8,
        #[serde(default)]
        drift_accelerated: bool,
        #[serde(default)]
        sanity_pause_count: u8,
        #[serde(default)]
        replace_count: u16,
        #[serde(default)]
        stalled: bool,
        #[serde(default)]
        phase_label: String,
        #[serde(default)]
        leg_count: u8,
        #[serde(default)]
        restart: bool,
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

        let bool_default = |field: &str| -> Result<bool, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(false),
            }
        };

        let u8_default = |field: &str, default: u8| -> Result<u8, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(default),
            }
        };

        let u16_default = |field: &str| -> Result<u16, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(0),
            }
        };

        let string_default = |field: &str| -> Result<String, D::Error> {
            match object.get(field) {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.{field}: {error}"))),
                _ => Ok(String::new()),
            }
        };

        let profile_default = || -> Result<DynamicMarketProfile, D::Error> {
            match object.get("profile") {
                Some(raw) if !raw.is_null() => serde_json::from_value(raw.clone())
                    .map_err(|error| D::Error::custom(format!("execution.profile: {error}"))),
                _ => Ok(DynamicMarketProfile::Standard),
            }
        };

        let execution = match kind {
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
                profile: profile_default()?,
                start_at_mid: bool_default("start_at_mid")?,
                initial_mid: match object.get("initial_mid") {
                    Some(raw) if !raw.is_null() => Some(serde_json::from_value(raw.clone()).map_err(
                        |error| D::Error::custom(format!("execution.initial_mid: {error}")),
                    )?),
                    _ => None,
                },
                initial_spread: match object.get("initial_spread") {
                    Some(raw) if !raw.is_null() => Some(serde_json::from_value(raw.clone()).map_err(
                        |error| D::Error::custom(format!("execution.initial_spread: {error}")),
                    )?),
                    _ => None,
                },
                step_index: u16_default("step_index")?,
                step_entered_at: datetime_default("step_entered_at")?,
                round: u8_default("round", 1)?,
                max_rounds: u8_default("max_rounds", 3)?,
                drift_accelerated: bool_default("drift_accelerated")?,
                sanity_pause_count: u8_default("sanity_pause_count", 0)?,
                replace_count: u16_default("replace_count")?,
                stalled: bool_default("stalled")?,
                phase_label: string_default("phase_label")?,
                leg_count: u8_default("leg_count", 0)?,
                restart: bool_default("restart")?,
            }),
            other => Err(D::Error::custom(format!(
                "unsupported execution.type: {other}"
            ))),
        }?;

        Ok(execution.normalized_prices())
    }
}

impl Execution {
    #[must_use]
    pub fn normalize_order_price(price: Decimal) -> Decimal {
        price.round_dp(2)
    }

    #[must_use]
    pub fn normalized_prices(&self) -> Self {
        match self {
            Self::Market => Self::Market,
            Self::Limit { limit_price } => Self::Limit {
                limit_price: Self::normalize_order_price(*limit_price),
            },
            Self::DynamicLimit {
                limit_price,
                start_price,
                end_price,
                current_percentage,
                percentage_step,
                interval_seconds,
                last_adjustment_time,
            } => Self::DynamicLimit {
                limit_price: Self::normalize_order_price(*limit_price),
                start_price: Self::normalize_order_price(*start_price),
                end_price: Self::normalize_order_price(*end_price),
                current_percentage: *current_percentage,
                percentage_step: *percentage_step,
                interval_seconds: *interval_seconds,
                last_adjustment_time: *last_adjustment_time,
            },
            Self::DynamicMarket { .. } => {
                let Some(mut state) = self.dynamic_market_state() else {
                    return self.clone();
                };
                state.limit_price = Self::normalize_order_price(state.limit_price);
                if let Some(mid) = state.initial_mid {
                    state.initial_mid = Some(Self::normalize_order_price(mid));
                }
                if let Some(spread) = state.initial_spread {
                    state.initial_spread = Some(Self::normalize_order_price(spread));
                }
                state.normalize_bounds();
                Self::from_dynamic_market_state(state)
            }
        }
    }

    pub fn dynamic_market_linear(
        limit_price: Decimal,
        start_percentage: f64,
        current_percentage: f64,
        percentage_step: f64,
        interval_seconds: i64,
    ) -> Self {
        Self::from_dynamic_market_state(DynamicMarketState::legacy(
            limit_price,
            start_percentage,
            current_percentage,
            percentage_step,
            interval_seconds,
            None,
        ))
    }

    pub fn dynamic_market_fast(
        limit_price: Decimal,
        start_percentage: f64,
        current_percentage: f64,
        percentage_step: f64,
        interval_seconds: i64,
    ) -> Self {
        let mut state = DynamicMarketState::legacy(
            limit_price,
            start_percentage,
            current_percentage,
            percentage_step,
            interval_seconds,
            None,
        );
        state.profile = DynamicMarketProfile::Fast;
        Self::from_dynamic_market_state(state)
    }

    pub fn dynamic_market_state(&self) -> Option<DynamicMarketState> {
        let Self::DynamicMarket {
            limit_price,
            start_percentage,
            current_percentage,
            percentage_step,
            interval_seconds,
            last_adjustment_time,
            profile,
            start_at_mid,
            initial_mid,
            initial_spread,
            step_index,
            step_entered_at,
            round,
            max_rounds,
            drift_accelerated,
            sanity_pause_count,
            replace_count,
            stalled,
            phase_label,
            leg_count,
            restart,
        } = self
        else {
            return None;
        };
        Some(DynamicMarketState {
            limit_price: *limit_price,
            start_percentage: *start_percentage,
            current_percentage: *current_percentage,
            percentage_step: *percentage_step,
            interval_seconds: *interval_seconds,
            last_adjustment_time: *last_adjustment_time,
            profile: *profile,
            start_at_mid: *start_at_mid,
            initial_mid: *initial_mid,
            initial_spread: *initial_spread,
            step_index: *step_index,
            step_entered_at: *step_entered_at,
            round: *round,
            max_rounds: *max_rounds,
            drift_accelerated: *drift_accelerated,
            sanity_pause_count: *sanity_pause_count,
            replace_count: *replace_count,
            stalled: *stalled,
            phase_label: phase_label.clone(),
            leg_count: *leg_count,
            restart: *restart,
        })
    }

    pub fn from_dynamic_market_state(state: DynamicMarketState) -> Self {
        Self::DynamicMarket {
            limit_price: state.limit_price,
            start_percentage: state.start_percentage,
            current_percentage: state.current_percentage,
            percentage_step: state.percentage_step,
            interval_seconds: state.interval_seconds,
            last_adjustment_time: state.last_adjustment_time,
            profile: state.profile,
            start_at_mid: state.start_at_mid,
            initial_mid: state.initial_mid,
            initial_spread: state.initial_spread,
            step_index: state.step_index,
            step_entered_at: state.step_entered_at,
            round: state.round,
            max_rounds: state.max_rounds,
            drift_accelerated: state.drift_accelerated,
            sanity_pause_count: state.sanity_pause_count,
            replace_count: state.replace_count,
            stalled: state.stalled,
            phase_label: state.phase_label,
            leg_count: state.leg_count,
            restart: state.restart,
        }
    }

    /// Price rung 0 and lock the spread benchmark before the broker submit.
    pub fn prepare_dynamic_market_submission(
        self,
        quote: DynamicMarketQuote,
        now: NaiveDateTime,
        positive_price: bool,
        leg_count: u8,
    ) -> Result<Self, Error> {
        let Some(mut state) = self.dynamic_market_state() else {
            return Ok(self);
        };
        if !install_submission_benchmark(&mut state, quote, now, positive_price, leg_count) {
            return Err(Error::InvalidRequest(
                "dynamic market submission is missing a usable quote".to_string(),
            ));
        }
        Ok(Self::from_dynamic_market_state(state).normalized_prices())
    }

    pub fn with_limit_price(self, limit_price: Decimal) -> Self {
        if let Some(mut state) = self.dynamic_market_state() {
            state.limit_price = limit_price;
            return Self::from_dynamic_market_state(state);
        }
        match self {
            Self::Market => Self::Market,
            Self::Limit { .. } => Self::Limit { limit_price },
            Self::DynamicLimit {
                start_price,
                end_price,
                current_percentage,
                percentage_step,
                interval_seconds,
                last_adjustment_time,
                ..
            } => Self::DynamicLimit {
                limit_price,
                start_price,
                end_price,
                current_percentage,
                percentage_step,
                interval_seconds,
                last_adjustment_time,
            },
            Self::DynamicMarket { .. } => self,
        }
    }

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
            } => (
                *current_percentage,
                Self::normalize_order_price(*limit_price).to_string(),
            ),
            Self::Limit { limit_price } => {
                (1.0, Self::normalize_order_price(*limit_price).to_string())
            }
            Self::Market => (1.0, "MARKET".to_string()),
        }
    }

    #[must_use]
    pub fn limit_price(&self) -> Option<Decimal> {
        match self {
            Self::Market => None,
            Self::Limit { limit_price }
            | Self::DynamicLimit { limit_price, .. }
            | Self::DynamicMarket { limit_price, .. } => {
                Some(Self::normalize_order_price(*limit_price))
            }
        }
    }

    fn interval_elapsed(
        last_adjustment_time: Option<NaiveDateTime>,
        interval_seconds: i64,
        now: NaiveDateTime,
    ) -> bool {
        match last_adjustment_time {
            Some(last_time) => {
                now.signed_duration_since(last_time).num_seconds() >= interval_seconds
            }
            None => true,
        }
    }

    pub fn advance_dynamic_limit(&self, now: NaiveDateTime) -> Result<Self, Error> {
        self.advance_dynamic_limit_inner(now, true)
    }

    pub fn continue_dynamic_limit(&self, now: NaiveDateTime) -> Result<Self, Error> {
        self.advance_dynamic_limit_inner(now, false)
    }

    fn advance_dynamic_limit_inner(
        &self,
        now: NaiveDateTime,
        require_interval: bool,
    ) -> Result<Self, Error> {
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
                let hanging = Self::normalize_order_price(*limit_price);
                let start_price = Self::normalize_order_price(*start_price);
                let end_price = Self::normalize_order_price(*end_price);

                if *current_percentage >= 1.0 {
                    return Ok(Self::Limit {
                        limit_price: end_price,
                    });
                }

                if require_interval
                    && !Self::interval_elapsed(
                        *last_adjustment_time,
                        *interval_seconds,
                        now,
                    )
                {
                    return Ok(self.normalized_prices());
                }

                let mut next_percentage = *current_percentage;
                loop {
                    let previous_percentage = next_percentage;
                    next_percentage = (next_percentage + *percentage_step).min(1.0);
                    if next_percentage >= 1.0 {
                        return Ok(Self::Limit {
                            limit_price: end_price,
                        });
                    }

                    let next_limit_price = Self::normalize_order_price(
                        start_price
                            + (end_price - start_price) * Self::progress_decimal(next_percentage)?,
                    );
                    let next_limit_price = if next_limit_price.is_zero() {
                        hanging
                    } else {
                        next_limit_price
                    };

                    // Two-decimal limit is the only effective price. Unchanged
                    // cents keep advancing in this interval; 100% becomes Limit.
                    if next_limit_price != hanging
                        || *percentage_step <= 0.0
                        || next_percentage <= previous_percentage
                    {
                        return Ok(Self::DynamicLimit {
                            limit_price: next_limit_price,
                            start_price,
                            end_price,
                            current_percentage: next_percentage,
                            percentage_step: *percentage_step,
                            interval_seconds: *interval_seconds,
                            last_adjustment_time: Some(now),
                        });
                    }
                }
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
        self.advance_dynamic_market_inner(best, worst, now, true)
    }

    pub fn continue_dynamic_market(
        &self,
        best: Decimal,
        worst: Decimal,
        now: NaiveDateTime,
    ) -> Result<Self, Error> {
        self.advance_dynamic_market_inner(best, worst, now, false)
    }

    pub fn step_dynamic_market(
        &self,
        quote: Option<DynamicMarketQuote>,
        now: NaiveDateTime,
        filled_qty: i32,
        options: DynamicMarketStepOptions,
    ) -> Result<DynamicMarketStepAction, Error> {
        let Some(state) = self.dynamic_market_state() else {
            return Err(Error::InvalidRequest(
                "step_dynamic_market() only supports dynamic_market execution".to_string(),
            ));
        };
        let outcome = step_dynamic_market_state(&state, quote, now, filled_qty, options);
        let execution = Self::from_dynamic_market_state(outcome.state).normalized_prices();
        Ok(match outcome.kind {
            DynamicMarketKind::Wait => DynamicMarketStepAction::Wait(execution),
            DynamicMarketKind::Replace => DynamicMarketStepAction::Replace(execution),
            DynamicMarketKind::Recreate => DynamicMarketStepAction::Recreate(execution),
            DynamicMarketKind::MarkStalled => DynamicMarketStepAction::MarkStalled(execution),
            DynamicMarketKind::MissingQuote => DynamicMarketStepAction::MissingQuote,
        })
    }

    fn advance_dynamic_market_inner(
        &self,
        best: Decimal,
        worst: Decimal,
        now: NaiveDateTime,
        require_interval: bool,
    ) -> Result<Self, Error> {
        if self.dynamic_market_state().is_none() {
            return Err(Error::InvalidRequest(
                "advance_dynamic_market() only supports dynamic_market execution".to_string(),
            ));
        }
        let action = self.step_dynamic_market(
            Some(DynamicMarketQuote { best, worst }),
            now,
            0,
            DynamicMarketStepOptions {
                force: !require_interval,
                ..DynamicMarketStepOptions::default()
            },
        )?;
        Ok(action
            .execution()
            .cloned()
            .unwrap_or_else(|| self.normalized_prices()))
    }

    #[must_use]
    pub fn submit_order_style(&self) -> SubmitOrderStyle {
        match self {
            Self::Market => SubmitOrderStyle::Market,
            Self::Limit { limit_price }
            | Self::DynamicLimit { limit_price, .. }
            | Self::DynamicMarket { limit_price, .. } => SubmitOrderStyle::Limit {
                limit_price: Self::normalize_order_price(*limit_price),
            },
        }
    }

    #[must_use]
    pub fn needs_market_retry(&self, _status: &str) -> bool {
        false
    }

    pub fn from_order_type(order_type: &str, limit_price: Option<Decimal>) -> Result<Self, Error> {
        match order_type.trim() {
            "market" => Ok(Self::Market),
            "limit" => Ok(Self::Limit {
                limit_price: Self::normalize_order_price(limit_price.ok_or_else(|| {
                    Error::InvalidRequest("limit order is missing limit_price".to_string())
                })?),
            }),
            _ => Ok(Self::Market),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_dynamic_market_json_keeps_defaults_and_does_not_require_new_fields() {
        let raw = r#"{
            "type":"dynamic_market",
            "limit_price":"1.16",
            "start_percentage":0.4,
            "current_percentage":0.7,
            "percentage_step":0.1,
            "interval_seconds":30,
            "last_adjustment_time":null
        }"#;
        let execution: Execution = serde_json::from_str(raw).expect("old json should parse");
        let state = execution.dynamic_market_state().expect("dynamic market");
        assert!(state.initial_mid.is_none());
        assert!(state.initial_spread.is_none());
        assert_eq!(state.current_percentage, 0.7);
        assert_eq!(state.round, 1);
        assert_eq!(state.max_rounds, 3);
        assert_eq!(state.profile, DynamicMarketProfile::Standard);
        assert!(!state.stalled);
        assert!(!state.start_at_mid);

        let stepped = execution
            .step_dynamic_market(
                Some(DynamicMarketQuote {
                    best: Decimal::new(100, 2),
                    worst: Decimal::new(120, 2),
                }),
                chrono::NaiveDateTime::parse_from_str("2026-04-17 10:00:00", "%Y-%m-%d %H:%M:%S")
                    .unwrap(),
                0,
                DynamicMarketStepOptions::default(),
            )
            .expect("step");
        let next = stepped.execution().expect("priced execution");
        let next_state = next.dynamic_market_state().unwrap();
        assert!(next_state.initial_mid.is_some());
        assert!(next_state.current_percentage + 1e-9 >= 0.6);
        assert!(!matches!(next, Execution::Market));
    }
}
