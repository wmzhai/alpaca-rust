use crate::DEFAULT_RISK_FREE_RATE;
use crate::contract;
use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::pricing;
use crate::snapshot;
use crate::types::{
    Greeks, OptionContract, OptionPosition, OptionRight, OptionStrategyCurvePoint,
    OptionStrategyInput, StrategyBreakEvenInput, StrategyBreakEvenSideInput, StrategyPnlInput,
    StrategyPnlPeak, StrategyPnlPeakSearchInput, StrategyPositionTotals,
};
use alpaca_time::clock;
use alpaca_time::expiration;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const CONTRACT_MULTIPLIER: f64 = 100.0;

mod decimal_number_contract {
    pub use alpaca_core::decimal::number_contract::deserialize;
    pub use alpaca_core::decimal::number_contract::serialize_decimal as serialize;
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionStrategy {
    #[serde(default)]
    #[ts(type = "Array<import('@alpaca/option').OptionPosition>")]
    pub positions: Vec<OptionPosition>,
    pub qty: i32,
    pub underlying_price: f64,
    #[serde(default)]
    #[ts(type = "import('@alpaca/option').Greeks")]
    pub greeks: Greeks,
    #[serde(with = "decimal_number_contract")]
    #[ts(type = "number")]
    pub cost: Decimal,
    #[serde(with = "decimal_number_contract")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[serde(with = "decimal_number_contract")]
    #[ts(type = "number")]
    pub pnl: Decimal,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub cashflow: Option<Decimal>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub spread: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub spread_rate: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub max_profit: Option<Decimal>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub max_loss: Option<Decimal>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub buying_power: Option<Decimal>,
    #[serde(default)]
    pub break_even_points: Vec<f64>,
    #[serde(default)]
    pub realtime_break_even_points: Vec<f64>,
    #[serde(default)]
    pub break_even_low_open: bool,
    #[serde(default)]
    pub break_even_high_open: bool,
    #[serde(default)]
    pub break_even_low_distance_percent: f64,
    #[serde(default)]
    pub break_even_high_distance_percent: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub break_even_width: Option<Decimal>,
    #[serde(default)]
    pub break_even_width_percent: f64,
    #[serde(default)]
    pub realtime_break_even_low_open: bool,
    #[serde(default)]
    pub realtime_break_even_high_open: bool,
    #[serde(default)]
    pub realtime_break_even_low_distance_percent: f64,
    #[serde(default)]
    pub realtime_break_even_high_distance_percent: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub realtime_break_even_width: Option<Decimal>,
    #[serde(default)]
    pub realtime_break_even_width_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub realtime_max_profit_price: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub realtime_max_profit: Option<Decimal>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alpaca_core::decimal::number_contract::option_decimal"
    )]
    #[ts(optional, type = "number")]
    pub realtime_max_profit_unit_value: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pnl_at_expire: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub short_expire_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub short_expiration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub long_expiration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub short_dte: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub long_dte: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub win_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub theta_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub theta_total: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rank: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub url: Option<String>,
    #[serde(skip)]
    #[ts(skip)]
    entry_cost: f64,
    #[serde(skip)]
    #[ts(skip)]
    dividend_yield: f64,
}

fn ensure_finite(code: &'static str, name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            code,
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn ensure_positive(code: &'static str, name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(code, name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            code,
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

impl OptionStrategy {
    fn position_contract(position: &OptionPosition) -> OptionResult<OptionContract> {
        contract::parse_occ_symbol(position.occ_symbol()).ok_or_else(|| {
            OptionError::new(
                "invalid_occ_symbol",
                format!("invalid occ symbol: {}", position.occ_symbol()),
            )
        })
    }

    fn validate_position(position: &OptionPosition, contract: &OptionContract) -> OptionResult<()> {
        ensure_positive(
            "invalid_strategy_payoff_input",
            "contract.strike",
            contract.strike,
        )?;
        if position.qty == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "quantity must not be zero",
            ));
        }
        ensure_finite(
            "invalid_strategy_payoff_input",
            "avg_cost",
            position.avg_cost(),
        )?;
        if let Some(implied_volatility) = position
            .snapshot_ref()
            .and_then(|snapshot| snapshot.implied_volatility)
        {
            ensure_finite(
                "invalid_strategy_payoff_input",
                "implied_volatility",
                implied_volatility,
            )?;
        }
        if let Some(snapshot) = position.snapshot_ref() {
            ensure_finite(
                "invalid_strategy_payoff_input",
                "mark_price",
                snapshot.price(),
            )?;
            ensure_finite(
                "invalid_strategy_payoff_input",
                "reference_underlying_price",
                snapshot.underlying_price(),
            )?;
        }
        Ok(())
    }

    fn valuation_years(expiration_date: &str, evaluation_time: &str) -> OptionResult<f64> {
        expiration::close(expiration_date).map_err(|error| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!("invalid expiration context for {expiration_date}: {error}"),
            )
        })?;
        Ok(expiration::years(
            expiration_date,
            Some(evaluation_time),
            None,
        ))
    }

    fn validate_strategy_qty(qty: i32) -> OptionResult<i32> {
        if qty > 0 {
            Ok(qty)
        } else {
            Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!("qty must be greater than zero: {qty}"),
            ))
        }
    }

    fn entry_cost_from_positions(
        positions: &[OptionPosition],
        qty: i32,
        entry_cost: Option<f64>,
    ) -> OptionResult<f64> {
        if let Some(entry_cost) = entry_cost {
            ensure_finite("invalid_strategy_payoff_input", "entry_cost", entry_cost)?;
            return Ok(entry_cost);
        }

        let mut total = 0.0;
        for position in positions {
            total += position.avg_cost() * f64::from(position.qty) * CONTRACT_MULTIPLIER;
        }
        Ok(total * f64::from(qty))
    }

    fn prepare_context(
        positions: &[OptionPosition],
        qty: i32,
        evaluation_time: &str,
        entry_cost: Option<f64>,
        dividend_yield: Option<f64>,
    ) -> OptionResult<(Vec<OptionPosition>, f64, f64)> {
        let dividend_yield = dividend_yield.unwrap_or(0.0);
        ensure_finite(
            "invalid_strategy_payoff_input",
            "dividend_yield",
            dividend_yield,
        )?;

        clock::parse_timestamp(evaluation_time).map_err(|error| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!("invalid evaluation_time: {error}"),
            )
        })?;

        let qty = Self::validate_strategy_qty(qty)?;
        let entry_cost = Self::entry_cost_from_positions(positions, qty, entry_cost)?;
        let mut prepared = Vec::with_capacity(positions.len());
        for position in positions {
            let contract = Self::position_contract(position)?;
            Self::validate_position(position, &contract)?;
            let years = Self::valuation_years(&contract.expiration_date, evaluation_time)?;
            if years > 0.0 {
                let implied_volatility = position
                    .snapshot_ref()
                    .and_then(|snapshot| snapshot.implied_volatility)
                    .ok_or_else(|| {
                        OptionError::new(
                            "invalid_strategy_payoff_input",
                            format!(
                                "implied_volatility is required before expiration: {}",
                                position.occ_symbol()
                            ),
                        )
                    })?;
                ensure_positive(
                    "invalid_strategy_payoff_input",
                    "implied_volatility",
                    implied_volatility,
                )?;
            }

            let mut prepared_position = position.clone();
            prepared_position.option_right = Some(contract.option_right);
            prepared_position.strike = Some(contract.strike);
            prepared_position.valuation_years = Some(years);
            prepared.push(prepared_position);
        }

        Ok((prepared, entry_cost, dividend_yield))
    }

    fn prepared_option_right(position: &OptionPosition) -> OptionResult<OptionRight> {
        position.option_right.clone().ok_or_else(|| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "option_right is required on prepared position: {}",
                    position.occ_symbol()
                ),
            )
        })
    }

    fn prepared_strike(position: &OptionPosition) -> OptionResult<f64> {
        position.strike.ok_or_else(|| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "strike is required on prepared position: {}",
                    position.occ_symbol()
                ),
            )
        })
    }

    fn prepared_years(position: &OptionPosition) -> OptionResult<f64> {
        position.valuation_years.ok_or_else(|| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "valuation_years is required on prepared position: {}",
                    position.occ_symbol()
                ),
            )
        })
    }

    fn prepared_implied_volatility(position: &OptionPosition) -> OptionResult<f64> {
        position
            .snapshot_ref()
            .and_then(|snapshot| snapshot.implied_volatility)
            .ok_or_else(|| {
                OptionError::new(
                    "invalid_strategy_payoff_input",
                    format!(
                        "implied_volatility is required before expiration: {}",
                        position.occ_symbol()
                    ),
                )
            })
    }

    fn mark_value_prepared(
        positions: &[OptionPosition],
        underlying_price: f64,
        dividend_yield: f64,
    ) -> OptionResult<f64> {
        ensure_finite(
            "invalid_strategy_payoff_input",
            "underlying_price",
            underlying_price,
        )?;
        if underlying_price < 0.0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!("underlying_price must be non-negative: {underlying_price}"),
            ));
        }
        ensure_finite(
            "invalid_strategy_payoff_input",
            "rate",
            DEFAULT_RISK_FREE_RATE,
        )?;

        let mut total = 0.0;
        for position in positions {
            let option_right = Self::prepared_option_right(position)?;
            let strike = Self::prepared_strike(position)?;
            let years = Self::prepared_years(position)?;
            let option_value = if years <= 0.0 {
                pricing::intrinsic_value(underlying_price, strike, option_right.as_str())?
            } else {
                pricing::price_black_scholes(&crate::BlackScholesInput {
                    spot: underlying_price,
                    strike,
                    years,
                    rate: DEFAULT_RISK_FREE_RATE,
                    dividend_yield,
                    volatility: Self::prepared_implied_volatility(position)?,
                    option_right,
                })?
            };
            total += option_value * f64::from(position.qty) * CONTRACT_MULTIPLIER;
        }
        Ok(total)
    }

    fn expiry_intrinsic_greeks(
        underlying_price: f64,
        strike: f64,
        option_right: &OptionRight,
    ) -> Greeks {
        let delta = match option_right {
            OptionRight::Call if underlying_price > strike => 1.0,
            OptionRight::Call if underlying_price < strike => 0.0,
            OptionRight::Call => 0.5,
            OptionRight::Put if underlying_price < strike => -1.0,
            OptionRight::Put if underlying_price > strike => 0.0,
            OptionRight::Put => -0.5,
        };

        Greeks {
            delta,
            ..Default::default()
        }
    }

    fn greeks_prepared(
        positions: &[OptionPosition],
        underlying_price: f64,
        dividend_yield: f64,
    ) -> OptionResult<Greeks> {
        ensure_positive(
            "invalid_strategy_payoff_input",
            "underlying_price",
            underlying_price,
        )?;
        ensure_finite(
            "invalid_strategy_payoff_input",
            "rate",
            DEFAULT_RISK_FREE_RATE,
        )?;

        let mut total = Greeks::default();
        for position in positions {
            let option_right = Self::prepared_option_right(position)?;
            let strike = Self::prepared_strike(position)?;
            let years = Self::prepared_years(position)?;
            let greeks = if years <= 0.0 {
                Self::expiry_intrinsic_greeks(underlying_price, strike, &option_right)
            } else {
                pricing::greeks_black_scholes(&crate::BlackScholesInput {
                    spot: underlying_price,
                    strike,
                    years,
                    rate: DEFAULT_RISK_FREE_RATE,
                    dividend_yield,
                    volatility: Self::prepared_implied_volatility(position)?,
                    option_right,
                })?
            };

            let quantity = f64::from(position.qty);
            total.delta += greeks.delta * quantity * CONTRACT_MULTIPLIER;
            total.gamma += greeks.gamma * quantity * CONTRACT_MULTIPLIER;
            total.vega += greeks.vega * quantity * CONTRACT_MULTIPLIER;
            total.theta += greeks.theta * quantity * CONTRACT_MULTIPLIER;
            total.rho += greeks.rho * quantity * CONTRACT_MULTIPLIER;
        }

        Ok(total)
    }

    fn position_totals_for(positions: &[OptionPosition], qty: i32) -> StrategyPositionTotals {
        let quantity = Decimal::from(qty);
        let quantity_abs = Decimal::from(qty.unsigned_abs());
        let mut value = Decimal::ZERO;
        let mut cost = Decimal::ZERO;
        let mut spread = Decimal::ZERO;

        for position in positions {
            value += position.value();
            cost += position.cost();

            let spread_per_contract =
                alpaca_core::decimal::from_f64(snapshot::spread(&position.snapshot), 2);
            spread += spread_per_contract
                * Decimal::from(position.qty.unsigned_abs())
                * Decimal::from(100);
        }

        value *= quantity;
        cost *= quantity;
        spread *= quantity_abs;

        let cost_f64 = cost.to_f64().unwrap_or(0.0);
        let spread_rate = if cost_f64.abs() > 1e-10 {
            let spread_f64 = spread.to_f64().unwrap_or(0.0);
            Some(spread_f64 / cost_f64.abs())
        } else {
            None
        };

        StrategyPositionTotals {
            value,
            cost,
            spread,
            spread_rate,
        }
    }

    fn validate_break_even_params(
        lower_bound: f64,
        upper_bound: f64,
        scan_step: Option<f64>,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<(f64, f64, usize)> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }

        let tolerance = tolerance.unwrap_or(1e-9);
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        let scan_step = scan_step.unwrap_or(1.0);
        ensure_positive("invalid_strategy_payoff_input", "scan_step", scan_step)?;
        let max_iterations = max_iterations.unwrap_or(100);
        if max_iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_iterations must be greater than zero",
            ));
        }

        Ok((tolerance, scan_step, max_iterations))
    }

    fn validate_break_even_bracket_params(
        lower_bound: f64,
        upper_bound: f64,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<(f64, usize)> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }

        let tolerance = tolerance.unwrap_or(1e-9);
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        let max_iterations = max_iterations.unwrap_or(100);
        if max_iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_iterations must be greater than zero",
            ));
        }

        Ok((tolerance, max_iterations))
    }

    pub fn expiration_time(positions: &[OptionPosition]) -> OptionResult<String> {
        let mut expiration_dates = Vec::with_capacity(positions.len());
        for position in positions {
            let contract = Self::position_contract(position)?;
            if !contract.expiration_date.trim().is_empty() {
                expiration_dates.push(contract.expiration_date);
            }
        }
        let expiration_date = expiration_dates
            .iter()
            .map(String::as_str)
            .min()
            .ok_or_else(|| {
                OptionError::new(
                    "invalid_strategy_payoff_input",
                    "at least one position with expiration_date is required",
                )
            })?;

        expiration::close(expiration_date).map_err(|error| {
            OptionError::new(
                "invalid_strategy_payoff_input",
                format!("invalid expiration context for {expiration_date}: {error}"),
            )
        })
    }

    pub fn from_input(input: &OptionStrategyInput) -> OptionResult<Self> {
        let evaluation_time = input
            .evaluation_time
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .map(Ok)
            .unwrap_or_else(|| Self::expiration_time(&input.positions))?;

        Self::prepare(
            &input.positions,
            input.qty,
            &evaluation_time,
            input.entry_cost,
            input.dividend_yield,
        )
    }

    pub fn prepare(
        positions: &[OptionPosition],
        qty: i32,
        evaluation_time: &str,
        entry_cost: Option<f64>,
        dividend_yield: Option<f64>,
    ) -> OptionResult<Self> {
        let (prepared, entry_cost, dividend_yield) =
            Self::prepare_context(positions, qty, evaluation_time, entry_cost, dividend_yield)?;
        let mut strategy = Self {
            positions: prepared,
            qty,
            underlying_price: 0.0,
            greeks: Greeks::default(),
            cost: alpaca_core::decimal::from_f64(entry_cost, 2),
            value: Decimal::ZERO,
            pnl: Decimal::ZERO,
            cashflow: None,
            spread: None,
            spread_rate: None,
            max_profit: None,
            max_loss: None,
            buying_power: None,
            break_even_points: Vec::new(),
            realtime_break_even_points: Vec::new(),
            break_even_low_open: false,
            break_even_high_open: false,
            break_even_low_distance_percent: 0.0,
            break_even_high_distance_percent: 0.0,
            break_even_width: None,
            break_even_width_percent: 0.0,
            realtime_break_even_low_open: false,
            realtime_break_even_high_open: false,
            realtime_break_even_low_distance_percent: 0.0,
            realtime_break_even_high_distance_percent: 0.0,
            realtime_break_even_width: None,
            realtime_break_even_width_percent: 0.0,
            realtime_max_profit_price: None,
            realtime_max_profit: None,
            realtime_max_profit_unit_value: None,
            pnl_at_expire: None,
            short_expire_delta: None,
            short_expiration: None,
            long_expiration: None,
            short_dte: None,
            long_dte: None,
            win_rate: None,
            theta_rate: None,
            theta_total: None,
            score: None,
            rank: None,
            url: None,
            entry_cost,
            dividend_yield,
        };
        strategy.calculate_value();
        strategy.calculate_spread();
        strategy.calculate_pnl();
        Ok(strategy)
    }

    pub fn pnl_at(&self, underlying_price: f64) -> OptionResult<f64> {
        Ok(self.mark_value_at(underlying_price)? - self.entry_cost)
    }

    pub fn mark_value_at(&self, underlying_price: f64) -> OptionResult<f64> {
        Ok(
            Self::mark_value_prepared(&self.positions, underlying_price, self.dividend_yield)?
                * f64::from(self.qty),
        )
    }

    pub fn positions(&self) -> &[OptionPosition] {
        &self.positions
    }

    pub fn qty_or_one(&self) -> i32 {
        self.qty.max(1)
    }

    pub fn underlying_price_f64(&self) -> Option<f64> {
        (self.underlying_price.is_finite() && self.underlying_price > 0.0)
            .then_some(self.underlying_price)
    }

    fn effective_entry_cost(&self) -> Decimal {
        self.cashflow.map(|cashflow| -cashflow).unwrap_or(self.cost)
    }

    fn sync_entry_cost_from_state(&mut self) {
        self.entry_cost = self.effective_entry_cost().to_f64().unwrap_or(0.0);
    }

    pub fn calculate_position_totals(&mut self) -> StrategyPositionTotals {
        let totals = self.position_totals();
        self.value = totals.value;
        self.cost = totals.cost;
        self.entry_cost = totals.cost.to_f64().unwrap_or(0.0);
        self.spread = Some(totals.spread);
        self.spread_rate = totals.spread_rate;
        self.pnl = self.value - self.cost;
        totals
    }

    pub fn calculate_cost_from_positions(&mut self) -> Decimal {
        let totals = self.position_totals();
        self.cost = totals.cost;
        self.entry_cost = totals.cost.to_f64().unwrap_or(0.0);
        self.cost
    }

    pub fn calculate_value(&mut self) -> Decimal {
        self.value = self.position_totals().value;
        self.value
    }

    pub fn calculate_pnl(&mut self) -> Decimal {
        self.sync_entry_cost_from_state();
        self.pnl = self.value - self.effective_entry_cost();
        self.pnl
    }

    pub fn calculate_spread(&mut self) -> Option<Decimal> {
        let totals = self.position_totals();
        self.spread = Some(totals.spread);
        self.spread_rate = totals.spread_rate;
        self.spread
    }

    pub fn calculate_greeks(&mut self) -> OptionResult<Greeks> {
        if self.underlying_price > 0.0 {
            self.greeks = self.greeks_at(self.underlying_price)?;
        } else {
            self.greeks = Self::aggregate_snapshot_greeks(&self.positions, self.qty)?;
        }
        Ok(self.greeks.clone())
    }

    pub fn calculate_expire_pnl(&mut self) -> OptionResult<Option<f64>> {
        self.sync_entry_cost_from_state();
        if self.underlying_price <= 0.0 {
            self.pnl_at_expire = None;
            return Ok(None);
        }
        let pnl = self.pnl_at(self.underlying_price)?;
        self.pnl_at_expire = Some(pnl);
        Ok(self.pnl_at_expire)
    }

    pub fn calculate_short_expire_delta(&mut self) -> OptionResult<Option<f64>> {
        if self.underlying_price <= 0.0 {
            self.short_expire_delta = None;
            return Ok(None);
        }
        let mut delta = 0.0;
        for position in &self.positions {
            if position.qty >= 0 {
                continue;
            }
            let option_right = Self::prepared_option_right(position)?;
            let strike = Self::prepared_strike(position)?;
            let greeks =
                Self::expiry_intrinsic_greeks(self.underlying_price, strike, &option_right);
            delta +=
                greeks.delta * f64::from(position.qty) * CONTRACT_MULTIPLIER * f64::from(self.qty);
        }
        self.short_expire_delta = Some(delta);
        Ok(self.short_expire_delta)
    }

    pub fn calculate_break_even_points(
        &mut self,
        lower_bound: f64,
        upper_bound: f64,
        scan_step: Option<f64>,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<Vec<f64>> {
        self.sync_entry_cost_from_state();
        self.break_even_points = self.break_even_points(
            lower_bound,
            upper_bound,
            scan_step,
            tolerance,
            max_iterations,
        )?;
        Ok(self.break_even_points.clone())
    }

    pub fn calculate_realtime_break_even_points(
        &mut self,
        current_price: f64,
        left_boundary: f64,
        right_boundary: f64,
        step_hint: Option<f64>,
        tolerance: Option<f64>,
        max_search_steps: Option<usize>,
    ) -> OptionResult<Vec<f64>> {
        self.sync_entry_cost_from_state();
        let peak = self.pnl_peak_from_current(&StrategyPnlPeakSearchInput {
            current_price,
            step_hint,
            left_boundary,
            right_boundary,
            tolerance,
            max_search_steps,
        })?;
        let Some(peak) = peak else {
            self.realtime_break_even_points.clear();
            return Ok(Vec::new());
        };

        let tolerance = tolerance.unwrap_or(1e-6);
        let low = self.break_even_between(
            left_boundary,
            peak.spot,
            Some(tolerance),
            Some(max_search_steps.unwrap_or(100)),
        )?;
        let high = self.break_even_between(
            peak.spot,
            right_boundary,
            Some(tolerance),
            Some(max_search_steps.unwrap_or(100)),
        )?;
        self.realtime_break_even_points =
            unique_break_even_points([low, high].into_iter().flatten(), tolerance);
        Ok(self.realtime_break_even_points.clone())
    }

    fn prepared_greeks_at(&self, underlying_price: f64) -> OptionResult<Greeks> {
        Self::greeks_prepared(&self.positions, underlying_price, self.dividend_yield)
    }

    pub fn sample_curve(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        step: f64,
    ) -> OptionResult<Vec<OptionStrategyCurvePoint>> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound < 0.0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!("lower_bound must be non-negative: {lower_bound}"),
            ));
        }
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }
        ensure_positive("invalid_strategy_payoff_input", "step", step)?;

        let mut points = Vec::new();
        let mut underlying_price = lower_bound;
        loop {
            let mark_value = self.mark_value_at(underlying_price)?;
            points.push(OptionStrategyCurvePoint {
                underlying_price,
                mark_value,
                pnl: mark_value - self.entry_cost,
            });

            if underlying_price >= upper_bound {
                break;
            }

            let next = (underlying_price + step).min(upper_bound);
            if next <= underlying_price {
                return Err(OptionError::new(
                    "invalid_strategy_payoff_input",
                    "step did not advance strategy curve scan",
                ));
            }
            underlying_price = next;
        }

        Ok(points)
    }

    pub fn break_even_between(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<Option<f64>> {
        let (tolerance, max_iterations) = Self::validate_break_even_bracket_params(
            lower_bound,
            upper_bound,
            tolerance,
            max_iterations,
        )?;

        let lower_value = self.pnl_at(lower_bound)?;
        if lower_value.abs() <= tolerance {
            return Ok(Some(lower_bound));
        }

        let upper_value = self.pnl_at(upper_bound)?;
        if upper_value.abs() <= tolerance {
            return Ok(Some(upper_bound));
        }

        if lower_value.signum() == upper_value.signum() {
            return Ok(None);
        }

        numeric::refine_bracketed_root(
            lower_bound,
            upper_bound,
            |spot| self.pnl_at(spot),
            Some(tolerance),
            Some(max_iterations),
        )
        .map(Some)
    }

    pub fn find_break_even_left(
        &self,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        self.find_break_even_toward(input.pivot, input.boundary, -input.scan_step.abs(), input)
    }

    pub fn find_break_even_right(
        &self,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        self.find_break_even_toward(input.pivot, input.boundary, input.scan_step.abs(), input)
    }

    fn find_break_even_toward(
        &self,
        pivot: f64,
        boundary: f64,
        signed_step: f64,
        input: &StrategyBreakEvenSideInput,
    ) -> OptionResult<Option<f64>> {
        let tolerance = input.tolerance.unwrap_or(1e-6);
        let max_iterations = input.max_iterations.unwrap_or(100);

        ensure_finite("invalid_strategy_payoff_input", "pivot", pivot)?;
        ensure_finite("invalid_strategy_payoff_input", "boundary", boundary)?;
        ensure_positive(
            "invalid_strategy_payoff_input",
            "scan_step",
            input.scan_step.abs(),
        )?;
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        if max_iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_iterations must be greater than zero",
            ));
        }
        if (signed_step < 0.0 && boundary >= pivot) || (signed_step > 0.0 && boundary <= pivot) {
            return Ok(None);
        }

        let mut previous = pivot;
        let mut previous_value = self.pnl_at(previous)?;
        if previous_value.abs() <= tolerance {
            return Ok(Some(previous));
        }

        loop {
            let next = if signed_step < 0.0 {
                (previous + signed_step).max(boundary)
            } else {
                (previous + signed_step).min(boundary)
            };
            let next_value = self.pnl_at(next)?;
            if next_value.abs() <= tolerance {
                return Ok(Some(next));
            }
            if next_value.signum() != previous_value.signum() {
                let low = previous.min(next);
                let high = previous.max(next);
                return self.break_even_between(low, high, Some(tolerance), Some(max_iterations));
            }
            if next == boundary {
                break;
            }
            previous = next;
            previous_value = next_value;
        }

        Ok(None)
    }

    pub fn break_even_points(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        scan_step: Option<f64>,
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
    ) -> OptionResult<Vec<f64>> {
        let (tolerance, scan_step, max_iterations) = Self::validate_break_even_params(
            lower_bound,
            upper_bound,
            scan_step,
            tolerance,
            max_iterations,
        )?;

        let mut roots = Vec::new();
        let mut previous_spot = lower_bound;
        let mut previous_value = self.pnl_at(previous_spot)?;
        if previous_value.abs() <= tolerance {
            push_unique_root(&mut roots, previous_spot, tolerance * 10.0);
        }

        let mut current_spot = (previous_spot + scan_step).min(upper_bound);
        while current_spot <= upper_bound {
            let current_value = self.pnl_at(current_spot)?;
            let root = if current_value.abs() <= tolerance {
                Some(current_spot)
            } else if previous_value.abs() <= tolerance {
                Some(previous_spot)
            } else if previous_value.signum() != current_value.signum() {
                Some(numeric::refine_bracketed_root(
                    previous_spot,
                    current_spot,
                    |spot| self.pnl_at(spot),
                    Some(tolerance),
                    Some(max_iterations),
                )?)
            } else {
                None
            };

            if let Some(root) = root {
                push_unique_root(&mut roots, root, tolerance * 10.0);
            }

            if current_spot >= upper_bound {
                break;
            }
            previous_spot = current_spot;
            previous_value = current_value;
            current_spot = (current_spot + scan_step).min(upper_bound);
        }

        roots.sort_by(|left, right| left.partial_cmp(right).unwrap());
        Ok(roots)
    }

    pub fn maximize_pnl_in_range(
        &self,
        lower_bound: f64,
        upper_bound: f64,
        iterations: usize,
    ) -> OptionResult<StrategyPnlPeak> {
        ensure_finite("invalid_strategy_payoff_input", "lower_bound", lower_bound)?;
        ensure_finite("invalid_strategy_payoff_input", "upper_bound", upper_bound)?;
        if lower_bound >= upper_bound {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"
                ),
            ));
        }
        if iterations == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "iterations must be greater than zero",
            ));
        }

        let mut left = lower_bound;
        let mut right = upper_bound;
        for _ in 0..iterations {
            let third = (right - left) / 3.0;
            let mid_left = left + third;
            let mid_right = right - third;
            let left_value = self.pnl_at(mid_left)?;
            let right_value = self.pnl_at(mid_right)?;

            if left_value < right_value {
                left = mid_left;
            } else {
                right = mid_right;
            }
        }

        let spot = (left + right) / 2.0;
        let pnl = self.pnl_at(spot)?;
        ensure_finite("invalid_strategy_payoff_input", "peak.pnl", pnl)?;
        Ok(StrategyPnlPeak { spot, pnl })
    }

    pub fn pnl_peak_from_current(
        &self,
        input: &StrategyPnlPeakSearchInput,
    ) -> OptionResult<Option<StrategyPnlPeak>> {
        ensure_positive(
            "invalid_strategy_payoff_input",
            "current_price",
            input.current_price,
        )?;
        ensure_positive(
            "invalid_strategy_payoff_input",
            "left_boundary",
            input.left_boundary,
        )?;
        if input.left_boundary >= input.current_price {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "left_boundary must be less than current_price: {} >= {}",
                    input.left_boundary, input.current_price
                ),
            ));
        }
        ensure_finite(
            "invalid_strategy_payoff_input",
            "right_boundary",
            input.right_boundary,
        )?;
        if input.right_boundary <= input.current_price {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "right_boundary must be greater than current_price: {} <= {}",
                    input.right_boundary, input.current_price
                ),
            ));
        }
        if input.right_boundary <= input.left_boundary {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                format!(
                    "right_boundary must be greater than left_boundary: {} <= {}",
                    input.right_boundary, input.left_boundary
                ),
            ));
        }
        let tolerance = input.tolerance.unwrap_or(1e-6);
        ensure_positive("invalid_strategy_payoff_input", "tolerance", tolerance)?;
        let max_search_steps = input.max_search_steps.unwrap_or(512);
        if max_search_steps == 0 {
            return Err(OptionError::new(
                "invalid_strategy_payoff_input",
                "max_search_steps must be greater than zero",
            ));
        }

        let preferred_step = input
            .step_hint
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or((input.current_price * 0.005).clamp(0.1, 5.0))
            .clamp(0.05, input.current_price.max(1.0) * 0.05);
        let range = input.right_boundary - input.left_boundary;
        let preferred_intervals = (range / preferred_step).ceil().max(2.0) as usize;
        let intervals = preferred_intervals.min(max_search_steps.max(2));
        let scan_step = range / intervals as f64;
        let mut best_index = 0usize;
        let mut best_spot = input.left_boundary;
        let mut best_pnl = self.pnl_at(best_spot)?;

        for index in 1..=intervals {
            let spot = if index == intervals {
                input.right_boundary
            } else {
                input.left_boundary + scan_step * index as f64
            };
            let pnl = self.pnl_at(spot)?;
            if pnl > best_pnl + tolerance {
                best_index = index;
                best_spot = spot;
                best_pnl = pnl;
            }
        }

        let peak = if best_index == 0 || best_index == intervals {
            StrategyPnlPeak {
                spot: best_spot,
                pnl: best_pnl,
            }
        } else {
            let lower = input.left_boundary + scan_step * (best_index - 1) as f64;
            let upper = input.left_boundary + scan_step * (best_index + 1) as f64;
            self.maximize_pnl_in_range(lower, upper, 80)?
        };

        if peak.pnl <= tolerance {
            return Ok(None);
        }
        Ok(Some(peak))
    }

    pub fn aggregate_snapshot_greeks(
        positions: &[OptionPosition],
        qty: i32,
    ) -> OptionResult<Greeks> {
        let qty = Self::validate_strategy_qty(qty)?;
        let mut total = Greeks::default();

        for position in positions {
            let quantity = f64::from(position.qty);
            let greeks = position.snapshot.greeks_or_default();
            total.delta += greeks.delta * quantity * CONTRACT_MULTIPLIER;
            total.gamma += greeks.gamma * quantity * CONTRACT_MULTIPLIER;
            total.vega += greeks.vega * quantity * CONTRACT_MULTIPLIER;
            total.theta += greeks.theta * quantity * CONTRACT_MULTIPLIER;
            total.rho += greeks.rho * quantity * CONTRACT_MULTIPLIER;
        }

        let qty = f64::from(qty);
        total.delta *= qty;
        total.gamma *= qty;
        total.vega *= qty;
        total.theta *= qty;
        total.rho *= qty;

        Ok(total)
    }

    pub fn greeks_at(&self, underlying_price: f64) -> OptionResult<Greeks> {
        let mut total = self.prepared_greeks_at(underlying_price)?;

        let qty = f64::from(self.qty);
        total.delta *= qty;
        total.gamma *= qty;
        total.vega *= qty;
        total.theta *= qty;
        total.rho *= qty;

        Ok(total)
    }

    pub fn position_totals(&self) -> StrategyPositionTotals {
        Self::position_totals_for(&self.positions, self.qty)
    }

    pub fn aggregate_model_greeks(
        positions: &[OptionPosition],
        underlying_price: f64,
        evaluation_time: &str,
        dividend_yield: Option<f64>,
        qty: i32,
    ) -> OptionResult<Greeks> {
        Self::prepare(positions, qty, evaluation_time, Some(0.0), dividend_yield)?
            .greeks_at(underlying_price)
    }
}

fn push_unique_root(roots: &mut Vec<f64>, root: f64, tolerance: f64) {
    if roots
        .iter()
        .any(|existing| (*existing - root).abs() <= tolerance)
    {
        return;
    }
    roots.push(root);
}

pub fn unique_break_even_points(points: impl IntoIterator<Item = f64>, tolerance: f64) -> Vec<f64> {
    let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        1e-6
    };
    let mut unique = Vec::new();
    for point in points {
        if !point.is_finite() {
            continue;
        }
        if unique
            .iter()
            .any(|existing: &f64| (*existing - point).abs() <= tolerance * 10.0)
        {
            continue;
        }
        unique.push(point);
    }
    unique.sort_by(|left, right| left.total_cmp(right));
    unique
}

pub fn strategy_pnl(input: &StrategyPnlInput) -> OptionResult<f64> {
    OptionStrategy::prepare(
        &input.positions,
        input.qty,
        &input.evaluation_time,
        input.entry_cost,
        input.dividend_yield,
    )?
    .pnl_at(input.underlying_price)
}

pub fn strategy_break_even_points(input: &StrategyBreakEvenInput) -> OptionResult<Vec<f64>> {
    OptionStrategy::prepare(
        &input.positions,
        input.qty,
        &input.evaluation_time,
        input.entry_cost,
        input.dividend_yield,
    )?
    .break_even_points(
        input.lower_bound,
        input.upper_bound,
        input.scan_step,
        input.tolerance,
        input.max_iterations,
    )
}
