//! Generic strategy analysis. Callers choose the book, the clock, and which sections to run.
//! Nothing here names a strategy or reads a database.

use std::collections::BTreeSet;

use alpaca_time::clock;
use alpaca_time::session;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::error::{OptionError, OptionResult};
use crate::numeric;
use crate::option_strategy::OptionStrategy;
use crate::probability::{self, ChanceOfProfitInput, DirectedBreakEven};
use crate::rate::RiskFreeRatePoint;
use crate::types::{
    Greeks, OptionPosition, OptionRight, OptionStrategyCurvePoint, quote_mid_option_value,
};

const MONEY_SCALE: u32 = 2;
const COST_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 2);
const ZERO_THETA: f64 = 1e-9;
const NODE_DEDUPE: f64 = 1e-8;
const DEFAULT_SCAN_STEP: f64 = 1.0;
const DEFAULT_TOLERANCE: f64 = 1e-6;
const DEFAULT_MAX_ITERATIONS: usize = 80;
const DEFAULT_MAX_SAMPLES: usize = 4096;
const DEFAULT_THETA_PRECISION_SECONDS: i64 = 3600;
const DEFAULT_THETA_ITERATIONS: usize = 64;

fn invalid(message: impl Into<String>) -> OptionError {
    OptionError::new("invalid_analysis_input", message)
}

fn money(value: Decimal) -> String {
    let rounded = alpaca_core::decimal::round(value, MONEY_SCALE);
    if rounded == Decimal::ZERO {
        return "0.00".to_string();
    }
    format!("{rounded:.2}")
}

fn decimal_from_f64(value: f64) -> OptionResult<Decimal> {
    if !value.is_finite() {
        return Err(invalid(format!("money value must be finite: {value}")));
    }
    Ok(alpaca_core::decimal::from_f64(value, MONEY_SCALE))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSection {
    Mark,
    Greeks,
    BreakEven,
    Curve,
    Peak,
    Probability,
    ThetaZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionStatus {
    NotRequested,
    Ok,
    Unavailable,
    BudgetExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueSource {
    Mid,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeStatus {
    Roots,
    NoRootProfit,
    NoRootLoss,
    NoRootFlat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThetaZeroKind {
    Found,
    InitiallyNonPositive,
    NoCrossing,
    BudgetExhausted,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyAnalysisInput {
    pub positions: Vec<OptionPosition>,
    /// Copies of one structure. Absent means each leg `qty` is already the actual size.
    #[serde(default)]
    pub qty: Option<i32>,
    #[serde(default)]
    pub stock_qty: i32,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_decimal_from_string_or_number"
    )]
    pub stock_cashflow: Decimal,
    #[serde(default)]
    pub entry_cost: Option<f64>,
    #[serde(
        default,
        deserialize_with = "alpaca_core::decimal::deserialize_option_decimal_from_string_or_number"
    )]
    pub cashflow: Option<Decimal>,
    pub underlying_price: f64,
    pub evaluation_time: String,
    #[serde(default)]
    pub dividend_yield: Option<f64>,
    /// Flat Treasury rate for every maturity. Absent uses the active curve copied at prepare.
    #[serde(default)]
    pub rate: Option<f64>,
    /// Explicit curve. When set, it replaces both `rate` and the active curve.
    #[serde(default)]
    pub rate_curve: Option<Vec<RateCurvePoint>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateCurvePoint {
    pub years: f64,
    pub rate: f64,
}

/// `OptionStrategy` always multiplies by a structure count. An omitted qty uses 1
/// so the stored leg quantities are not scaled again.
fn structure_scale(qty: Option<i32>) -> OptionResult<i32> {
    match qty {
        None => Ok(1),
        Some(qty) if qty > 0 => Ok(qty),
        Some(qty) => Err(invalid(format!("qty must be greater than zero: {qty}"))),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceSearch {
    pub lower_bound: f64,
    pub upper_bound: f64,
    #[serde(default)]
    pub scan_step: Option<f64>,
    #[serde(default)]
    pub nodes: Vec<f64>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub max_samples: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbabilityRequest {
    pub horizon_days: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThetaZeroSearch {
    pub start_time: String,
    pub end_time: String,
    #[serde(default)]
    pub precision_seconds: Option<i64>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationRequest {
    pub evaluation_time: String,
    #[serde(default)]
    pub iv_multiplier: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StrategyAnalysisOptions {
    #[serde(default)]
    pub sections: Vec<AnalysisSection>,
    #[serde(default)]
    pub price_search: Option<PriceSearch>,
    #[serde(default)]
    pub probability: Option<ProbabilityRequest>,
    #[serde(default)]
    pub time_search: Option<ThetaZeroSearch>,
    #[serde(default)]
    pub evaluations: Vec<EvaluationRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectedRoot {
    pub at: f64,
    pub direction: i8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfitRegion {
    pub lower: f64,
    pub upper: f64,
    pub lower_is_window: bool,
    pub upper_is_window: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakEvenSection {
    pub points: Vec<DirectedRoot>,
    pub touches: Vec<f64>,
    pub profit_regions: Vec<ProfitRegion>,
    pub range_status: RangeStatus,
    pub lower_truncated: bool,
    pub upper_truncated: bool,
    pub lower_unbounded: bool,
    pub upper_unbounded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeakSection {
    pub spot: f64,
    pub pnl: f64,
    pub at_boundary: bool,
    pub window_limited: bool,
    pub non_positive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThetaZeroSection {
    pub kind: ThetaZeroKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_from_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_to_end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyAnalysis {
    pub cost: String,
    pub option_cost: String,
    pub stock_cost: String,
    pub value: Option<String>,
    pub value_source: Option<ValueSource>,
    pub model_value: Option<String>,
    pub mid_value: Option<String>,
    pub pnl: Option<String>,
    pub mark_status: SectionStatus,
    pub greeks: Option<Greeks>,
    pub greeks_status: SectionStatus,
    pub break_even: Option<BreakEvenSection>,
    pub break_even_status: SectionStatus,
    pub curve: Option<Vec<OptionStrategyCurvePoint>>,
    pub curve_status: SectionStatus,
    pub peak: Option<PeakSection>,
    pub peak_status: SectionStatus,
    pub probability: Option<f64>,
    pub probability_status: SectionStatus,
    pub theta_zero: Option<ThetaZeroSection>,
    pub theta_zero_status: SectionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evaluations: Vec<StrategyAnalysis>,
}

struct ResolvedBook {
    option_cost: Decimal,
    stock_cost: Decimal,
    total_cost: Decimal,
}

pub fn analyze_strategy(
    input: &StrategyAnalysisInput,
    options: &StrategyAnalysisOptions,
) -> OptionResult<StrategyAnalysis> {
    analyze_strategy_inner(input, options, true)
}

fn analyze_strategy_inner(
    input: &StrategyAnalysisInput,
    options: &StrategyAnalysisOptions,
    include_evaluations: bool,
) -> OptionResult<StrategyAnalysis> {
    let requested = requested_sections(options);
    let wants = |section: AnalysisSection| requested.contains(&section);
    let book = resolve_book(input)?;
    let strategy = build_strategy(input, &book)?;

    let mut analysis = empty_analysis(&book);
    if wants(AnalysisSection::Mark) {
        fill_mark(&mut analysis, input, &strategy, &book)?;
    }
    if wants(AnalysisSection::Greeks) {
        analysis.greeks = Some(strategy.greeks_at(input.underlying_price)?);
        analysis.greeks_status = SectionStatus::Ok;
    }
    if wants(AnalysisSection::BreakEven)
        || wants(AnalysisSection::Curve)
        || wants(AnalysisSection::Peak)
        || wants(AnalysisSection::Probability)
    {
        let search = options
            .price_search
            .as_ref()
            .ok_or_else(|| invalid("price_search is required for break_even, curve, peak, or probability"))?;
        validate_search(search)?;
        let step = search.scan_step.unwrap_or(DEFAULT_SCAN_STEP);
        let max_samples = search.max_samples.unwrap_or(DEFAULT_MAX_SAMPLES);
        if sample_count(search.lower_bound, search.upper_bound, step) > max_samples {
            if wants(AnalysisSection::BreakEven) {
                analysis.break_even_status = SectionStatus::BudgetExhausted;
            }
            if wants(AnalysisSection::Curve) {
                analysis.curve_status = SectionStatus::BudgetExhausted;
            }
            if wants(AnalysisSection::Peak) {
                analysis.peak_status = SectionStatus::BudgetExhausted;
            }
            if wants(AnalysisSection::Probability) {
                analysis.probability_status = SectionStatus::BudgetExhausted;
            }
        } else {
            if wants(AnalysisSection::BreakEven) || wants(AnalysisSection::Probability) {
                let section = break_even_section(&strategy, search)?;
                if wants(AnalysisSection::Probability) {
                    fill_probability(&mut analysis, input, options, &strategy, search, &section)?;
                }
                if wants(AnalysisSection::BreakEven) {
                    analysis.break_even = Some(section);
                    analysis.break_even_status = SectionStatus::Ok;
                }
            }
            if wants(AnalysisSection::Curve) {
                analysis.curve = Some(strategy.sample_curve(search.lower_bound, search.upper_bound, step)?);
                analysis.curve_status = SectionStatus::Ok;
            }
            if wants(AnalysisSection::Peak) {
                analysis.peak = Some(peak_section(&strategy, search)?);
                analysis.peak_status = SectionStatus::Ok;
            }
        }
    }
    if wants(AnalysisSection::ThetaZero) {
        let search = options
            .time_search
            .as_ref()
            .ok_or_else(|| invalid("time_search is required for theta_zero"))?;
        let section = find_theta_zero(search, |timestamp| {
            let mut timed = input.clone();
            timed.evaluation_time = timestamp.to_string();
            build_strategy(&timed, &book).ok().and_then(|timed_strategy| {
                timed_strategy
                    .greeks_at(input.underlying_price)
                    .ok()
                    .map(|greeks| greeks.theta)
                    .filter(|theta| theta.is_finite())
            })
        })?;
        analysis.theta_zero_status = match section.kind {
            ThetaZeroKind::Found => SectionStatus::Ok,
            ThetaZeroKind::BudgetExhausted => SectionStatus::BudgetExhausted,
            ThetaZeroKind::Unavailable => SectionStatus::Unavailable,
            ThetaZeroKind::InitiallyNonPositive | ThetaZeroKind::NoCrossing => SectionStatus::Ok,
        };
        analysis.theta_zero = Some(section);
    }

    if include_evaluations {
        for evaluation in &options.evaluations {
            let mut next_input = input.clone();
            next_input.evaluation_time = evaluation.evaluation_time.clone();
            if let Some(multiplier) = evaluation.iv_multiplier {
                if !multiplier.is_finite() || multiplier <= 0.0 {
                    return Err(invalid(format!(
                        "iv_multiplier must be positive: {multiplier}"
                    )));
                }
                for position in &mut next_input.positions {
                    if let Some(iv) = position.snapshot.implied_volatility {
                        position.snapshot.implied_volatility = Some(iv * multiplier);
                    }
                }
            }
            let mut next_options = options.clone();
            next_options.evaluations.clear();
            analysis
                .evaluations
                .push(analyze_strategy_inner(&next_input, &next_options, false)?);
        }
    }

    Ok(analysis)
}

fn requested_sections(options: &StrategyAnalysisOptions) -> BTreeSet<AnalysisSection> {
    if options.sections.is_empty() {
        BTreeSet::from([AnalysisSection::Mark, AnalysisSection::Greeks])
    } else {
        options.sections.iter().copied().collect()
    }
}

fn resolve_book(input: &StrategyAnalysisInput) -> OptionResult<ResolvedBook> {
    let scale = structure_scale(input.qty)?;
    if !input.underlying_price.is_finite() || input.underlying_price < 0.0 {
        return Err(invalid(format!(
            "underlying_price must be non-negative: {}",
            input.underlying_price
        )));
    }
    if input.rate.is_some() && input.rate_curve.is_some() {
        return Err(invalid("rate and rate_curve cannot both be set"));
    }
    if let Some(rate) = input.rate {
        if !rate.is_finite() || rate <= 0.0 || rate >= 1.0 {
            return Err(invalid(format!("rate must be in (0, 1): {rate}")));
        }
    }
    if let Some(curve) = &input.rate_curve {
        if curve.len() < 2 {
            return Err(invalid("rate_curve must contain at least two points"));
        }
    }
    let stock_cost = -input.stock_cashflow;
    let from_legs = leg_option_cost(&input.positions, scale);
    let (option_cost, total_cost) = match (input.cashflow, input.entry_cost) {
        (Some(cashflow), Some(entry_cost)) => {
            let from_cashflow = -cashflow;
            let from_entry = decimal_from_f64(entry_cost)? - input.stock_cashflow;
            if (from_cashflow - from_entry).abs() > COST_TOLERANCE {
                return Err(OptionError::new(
                    "inconsistent_strategy_cost",
                    "cashflow and entry_cost do not describe the same total cost",
                ));
            }
            (from_cashflow - stock_cost, from_cashflow)
        }
        (Some(cashflow), None) => {
            let total = -cashflow;
            (total - stock_cost, total)
        }
        (None, Some(entry_cost)) => {
            let option_cost = decimal_from_f64(entry_cost)?;
            (option_cost, option_cost + stock_cost)
        }
        (None, None) => (from_legs, from_legs + stock_cost),
    };
    Ok(ResolvedBook {
        option_cost,
        stock_cost,
        total_cost,
    })
}

fn leg_option_cost(positions: &[OptionPosition], qty: i32) -> Decimal {
    let mut total = Decimal::ZERO;
    for position in positions {
        total += position.avg_cost * Decimal::from(position.qty) * Decimal::from(100);
    }
    total * Decimal::from(qty)
}

fn build_strategy(input: &StrategyAnalysisInput, book: &ResolvedBook) -> OptionResult<OptionStrategy> {
    let option_cost = book
        .option_cost
        .to_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid("option cost does not fit a finite entry_cost"))?;
    let scale = structure_scale(input.qty)?;
    let mut strategy = OptionStrategy::prepare(
        &input.positions,
        scale,
        input.evaluation_time.trim(),
        Some(option_cost),
        input.dividend_yield,
    )?;
    strategy.cost = book.option_cost;
    strategy.stock_qty = input.stock_qty;
    strategy.stock_cashflow = input.stock_cashflow;
    strategy.cashflow = Some(-book.total_cost);
    strategy.underlying_price = input.underlying_price;
    if let Some(curve) = &input.rate_curve {
        let points = curve
            .iter()
            .map(|point| RiskFreeRatePoint {
                years: point.years,
                rate: point.rate,
            })
            .collect::<Vec<_>>();
        strategy.use_rate_curve(&points)?;
    } else if let Some(rate) = input.rate {
        strategy.use_rate_curve(&[
            RiskFreeRatePoint {
                years: 1.0 / 12.0,
                rate,
            },
            RiskFreeRatePoint { years: 30.0, rate },
        ])?;
    }
    Ok(strategy)
}

fn empty_analysis(book: &ResolvedBook) -> StrategyAnalysis {
    StrategyAnalysis {
        cost: money(book.total_cost),
        option_cost: money(book.option_cost),
        stock_cost: money(book.stock_cost),
        value: None,
        value_source: None,
        model_value: None,
        mid_value: None,
        pnl: None,
        mark_status: SectionStatus::NotRequested,
        greeks: None,
        greeks_status: SectionStatus::NotRequested,
        break_even: None,
        break_even_status: SectionStatus::NotRequested,
        curve: None,
        curve_status: SectionStatus::NotRequested,
        peak: None,
        peak_status: SectionStatus::NotRequested,
        probability: None,
        probability_status: SectionStatus::NotRequested,
        theta_zero: None,
        theta_zero_status: SectionStatus::NotRequested,
        evaluations: Vec::new(),
    }
}

fn fill_mark(
    analysis: &mut StrategyAnalysis,
    input: &StrategyAnalysisInput,
    strategy: &OptionStrategy,
    book: &ResolvedBook,
) -> OptionResult<()> {
    let model = decimal_from_f64(strategy.mark_value_at(input.underlying_price)?)?;
    let stock_value = decimal_from_f64(f64::from(input.stock_qty) * input.underlying_price)?;
    let mid = quote_mid_option_value(&input.positions, structure_scale(input.qty)?).map(|option_value| option_value + stock_value);
    let regular = session::is_regular_session_at(input.evaluation_time.trim());
    let (value, source) = if regular {
        if let Some(mid_value) = mid {
            (mid_value, ValueSource::Mid)
        } else {
            (model, ValueSource::Model)
        }
    } else {
        (model, ValueSource::Model)
    };
    analysis.model_value = Some(money(model));
    analysis.mid_value = mid.map(|value| money(value));
    analysis.value = Some(money(value));
    analysis.value_source = Some(source);
    analysis.pnl = Some(money(value - book.total_cost));
    analysis.mark_status = SectionStatus::Ok;
    Ok(())
}

fn validate_search(search: &PriceSearch) -> OptionResult<()> {
    if !search.lower_bound.is_finite() || search.lower_bound < 0.0 {
        return Err(invalid(format!(
            "lower_bound must be non-negative: {}",
            search.lower_bound
        )));
    }
    if !search.upper_bound.is_finite() || search.lower_bound >= search.upper_bound {
        return Err(invalid(format!(
            "lower_bound must be less than upper_bound: {} >= {}",
            search.lower_bound, search.upper_bound
        )));
    }
    let step = search.scan_step.unwrap_or(DEFAULT_SCAN_STEP);
    if !step.is_finite() || step <= 0.0 {
        return Err(invalid(format!("scan_step must be positive: {step}")));
    }
    if let Some(tolerance) = search.tolerance {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(invalid(format!("tolerance must be positive: {tolerance}")));
        }
    }
    if let Some(max_iterations) = search.max_iterations {
        if max_iterations == 0 {
            return Err(invalid("max_iterations must be greater than zero"));
        }
    }
    if let Some(max_samples) = search.max_samples {
        if max_samples == 0 {
            return Err(invalid("max_samples must be greater than zero"));
        }
    }
    Ok(())
}

fn sample_count(lower: f64, upper: f64, step: f64) -> usize {
    ((upper - lower) / step).ceil() as usize + 1
}

fn turning_nodes(
    strategy: &OptionStrategy,
    nodes: &[f64],
    tolerance: f64,
    max_iterations: usize,
) -> Vec<f64> {
    let mut extra = Vec::new();
    for pair in nodes.windows(2) {
        let left = pair[0];
        let right = pair[1];
        if right <= left + tolerance {
            continue;
        }
        let Ok(left_greeks) = strategy.greeks_at(left) else {
            continue;
        };
        let Ok(right_greeks) = strategy.greeks_at(right) else {
            continue;
        };
        let left_delta = left_greeks.delta;
        let right_delta = right_greeks.delta;
        if !left_delta.is_finite()
            || !right_delta.is_finite()
            || left_delta.abs() <= tolerance
            || right_delta.abs() <= tolerance
            || left_delta.signum() == right_delta.signum()
        {
            continue;
        }
        if let Ok(at) = numeric::refine_bracketed_root(
            left,
            right,
            |price| strategy.greeks_at(price).map(|greeks| greeks.delta),
            Some(tolerance.max(1e-8)),
            Some(max_iterations),
        ) {
            extra.push(at);
        }
    }
    extra
}

fn merge_nodes(mut nodes: Vec<f64>, extra: &[f64]) -> Vec<f64> {
    nodes.extend_from_slice(extra);
    nodes.retain(|value| value.is_finite());
    nodes.sort_by(|left, right| left.total_cmp(right));
    let mut unique = Vec::with_capacity(nodes.len());
    for value in nodes {
        if unique
            .last()
            .is_none_or(|previous: &f64| (previous - value).abs() > NODE_DEDUPE)
        {
            unique.push(value);
        }
    }
    unique
}

fn search_nodes(strategy: &OptionStrategy, search: &PriceSearch) -> Vec<f64> {
    let mut nodes = vec![search.lower_bound, search.upper_bound];
    nodes.extend(search.nodes.iter().copied());
    for position in strategy.positions() {
        if let Some(strike) = position.strike {
            if strike >= search.lower_bound && strike <= search.upper_bound {
                nodes.push(strike);
            }
        }
    }
    let step = search.scan_step.unwrap_or(DEFAULT_SCAN_STEP);
    let mut spot = search.lower_bound;
    while spot < search.upper_bound {
        spot = (spot + step).min(search.upper_bound);
        nodes.push(spot);
    }
    nodes.retain(|value| value.is_finite());
    nodes.sort_by(|left, right| left.total_cmp(right));
    let mut unique = Vec::with_capacity(nodes.len());
    for value in nodes {
        if unique
            .last()
            .is_none_or(|previous: &f64| (previous - value).abs() > NODE_DEDUPE)
        {
            unique.push(value);
        }
    }
    unique
}

fn break_even_section(
    strategy: &OptionStrategy,
    search: &PriceSearch,
) -> OptionResult<BreakEvenSection> {
    let tolerance = search.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    let max_iterations = search.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
    let sampled = search_nodes(strategy, search);
    let nodes = merge_nodes(
        sampled.clone(),
        &turning_nodes(strategy, &sampled, tolerance, max_iterations),
    );
    let max_samples = search.max_samples.unwrap_or(DEFAULT_MAX_SAMPLES);
    if nodes.len() > max_samples {
        return Err(invalid(
            "break-even node count exceeds max_samples; raise the budget or widen scan_step",
        ));
    }
    let mut touches = Vec::new();
    for price in &nodes {
        let pnl = strategy.pnl_at(*price)?;
        if pnl.abs() <= tolerance {
            touches.push(*price);
        }
    }
    let directed = probability::polished_directed_break_evens(
        &nodes,
        |price| strategy.pnl_at(price).unwrap_or(f64::NAN),
        tolerance,
        max_iterations,
    )?;
    let mut points = directed
        .into_iter()
        .map(|point| DirectedRoot {
            at: point.at,
            direction: point.direction,
        })
        .collect::<Vec<_>>();
    let duplicate_gap = tolerance.max(1e-4);
    points.dedup_by(|right, left| (left.at - right.at).abs() <= duplicate_gap);
    let (profit_regions, range_status) = profit_regions(strategy, search, &points)?;
    let (lower_slope, upper_slope) = terminal_slopes(strategy);
    let lower_truncated = profit_regions.iter().any(|region| region.lower_is_window);
    let upper_truncated = profit_regions.iter().any(|region| region.upper_is_window);
    Ok(BreakEvenSection {
        points,
        touches,
        profit_regions,
        range_status,
        lower_truncated,
        upper_truncated,
        lower_unbounded: lower_slope < 0.0,
        upper_unbounded: upper_slope > 0.0,
    })
}

fn profit_regions(
    strategy: &OptionStrategy,
    search: &PriceSearch,
    points: &[DirectedRoot],
) -> OptionResult<(Vec<ProfitRegion>, RangeStatus)> {
    let tolerance = search.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    if points.is_empty() {
        let mid = (search.lower_bound + search.upper_bound) / 2.0;
        let pnl = strategy.pnl_at(mid)?;
        let status = if pnl > tolerance {
            RangeStatus::NoRootProfit
        } else if pnl < -tolerance {
            RangeStatus::NoRootLoss
        } else {
            RangeStatus::NoRootFlat
        };
        let regions = if status == RangeStatus::NoRootProfit {
            vec![ProfitRegion {
                lower: search.lower_bound,
                upper: search.upper_bound,
                lower_is_window: true,
                upper_is_window: true,
            }]
        } else {
            Vec::new()
        };
        return Ok((regions, status));
    }

    let mut regions = Vec::new();
    let mut in_profit = points[0].direction < 0;
    let mut left = search.lower_bound;
    for point in points {
        if in_profit && point.at > left {
            regions.push(ProfitRegion {
                lower: left,
                upper: point.at,
                lower_is_window: (left - search.lower_bound).abs() <= NODE_DEDUPE,
                upper_is_window: false,
            });
        }
        in_profit = !in_profit;
        left = point.at;
    }
    if in_profit && search.upper_bound > left {
        regions.push(ProfitRegion {
            lower: left,
            upper: search.upper_bound,
            lower_is_window: false,
            upper_is_window: true,
        });
    }
    Ok((regions, RangeStatus::Roots))
}

fn terminal_slopes(strategy: &OptionStrategy) -> (f64, f64) {
    let mut call_qty = 0.0;
    let mut put_qty = 0.0;
    for position in strategy.positions() {
        let right = position
            .option_right
            .clone()
            .unwrap_or_else(|| position.contract_info().option_right);
        let signed = f64::from(position.qty) * f64::from(strategy.qty);
        match right {
            OptionRight::Call => call_qty += signed,
            OptionRight::Put => put_qty += signed,
        }
    }
    let stock = f64::from(strategy.stock_qty);
    (stock - 100.0 * put_qty, stock + 100.0 * call_qty)
}

struct RefinedExtrema {
    max_spot: f64,
    max_value: f64,
    min_value: f64,
    tolerance: f64,
}

fn refined_extrema(strategy: &OptionStrategy, search: &PriceSearch) -> OptionResult<RefinedExtrema> {
    let step = search.scan_step.unwrap_or(DEFAULT_SCAN_STEP);
    let coarse = numeric::scan_range_extrema(
        search.lower_bound,
        search.upper_bound,
        Some(step),
        |price| strategy.pnl_at(price),
    )?;
    let tolerance = search.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    let max_iterations = search.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
    let sampled = search_nodes(strategy, search);
    let mut max_spot = coarse.max_spot;
    let mut max_value = coarse.max_value;
    let mut min_value = coarse.min_value;
    for price in turning_nodes(strategy, &sampled, tolerance, max_iterations) {
        let value = strategy.pnl_at(price)?;
        if value > max_value {
            max_spot = price;
            max_value = value;
        }
        if value < min_value {
            min_value = value;
        }
    }
    Ok(RefinedExtrema {
        max_spot,
        max_value,
        min_value,
        tolerance,
    })
}

fn peak_section(strategy: &OptionStrategy, search: &PriceSearch) -> OptionResult<PeakSection> {
    let extrema = refined_extrema(strategy, search)?;
    Ok(PeakSection {
        spot: extrema.max_spot,
        pnl: extrema.max_value,
        at_boundary: (extrema.max_spot - search.lower_bound).abs() <= extrema.tolerance
            || (extrema.max_spot - search.upper_bound).abs() <= extrema.tolerance,
        window_limited: true,
        non_positive: extrema.max_value <= extrema.tolerance,
    })
}

fn fill_probability(
    analysis: &mut StrategyAnalysis,
    input: &StrategyAnalysisInput,
    options: &StrategyAnalysisOptions,
    strategy: &OptionStrategy,
    search: &PriceSearch,
    section: &BreakEvenSection,
) -> OptionResult<()> {
    let Some(request) = options.probability.as_ref() else {
        analysis.probability_status = SectionStatus::Unavailable;
        return Ok(());
    };
    if !request.horizon_days.is_finite()
        || request.horizon_days <= 0.0
        || !request.volatility.is_finite()
        || request.volatility <= 0.0
        || input.underlying_price <= 0.0
    {
        analysis.probability_status = SectionStatus::Unavailable;
        return Ok(());
    }
    let extrema = refined_extrema(strategy, search)?;
    let breakevens = section
        .points
        .iter()
        .map(|point| DirectedBreakEven {
            at: point.at,
            direction: point.direction,
        })
        .collect::<Vec<_>>();
    let pnl_without = strategy.pnl_at(input.underlying_price)?;
    analysis.probability = Some(probability::chance_of_profit(ChanceOfProfitInput {
        spot: input.underlying_price,
        days: request.horizon_days,
        volatility: request.volatility,
        breakevens: &breakevens,
        max_profit: extrema.max_value,
        min_pnl: extrema.min_value,
        pnl_without_break_even: pnl_without,
    })?);
    analysis.probability_status = SectionStatus::Ok;
    Ok(())
}

pub fn find_theta_zero<F>(search: &ThetaZeroSearch, mut theta_at: F) -> OptionResult<ThetaZeroSection>
where
    F: FnMut(&str) -> Option<f64>,
{
    let precision = search
        .precision_seconds
        .unwrap_or(DEFAULT_THETA_PRECISION_SECONDS);
    let max_iterations = search.max_iterations.unwrap_or(DEFAULT_THETA_ITERATIONS);
    if precision <= 0 {
        return Err(invalid(format!(
            "precision_seconds must be positive: {precision}"
        )));
    }
    if max_iterations == 0 {
        return Err(invalid("max_iterations must be greater than zero"));
    }
    let start_time = clock::parse_timestamp(search.start_time.trim()).map_err(|error| {
        invalid(format!("invalid theta zero start_time: {error}"))
    })?;
    let end_time = clock::parse_timestamp(search.end_time.trim())
        .map_err(|error| invalid(format!("invalid theta zero end_time: {error}")))?;
    let start_unix = clock::unix_seconds(&start_time)
        .map_err(|error| invalid(format!("invalid theta zero start_time: {error}")))?;
    let end_unix = clock::unix_seconds(&end_time)
        .map_err(|error| invalid(format!("invalid theta zero end_time: {error}")))?;
    if end_unix <= start_unix {
        return Ok(ThetaZeroSection {
            kind: ThetaZeroKind::NoCrossing,
            at: None,
            days_from_start: None,
            days_to_end: None,
        });
    }

    let initial = theta_at(&start_time);
    let Some(initial) = initial.filter(|theta| theta.is_finite()) else {
        return Ok(ThetaZeroSection {
            kind: ThetaZeroKind::Unavailable,
            at: None,
            days_from_start: None,
            days_to_end: None,
        });
    };
    if initial <= ZERO_THETA {
        return Ok(ThetaZeroSection {
            kind: ThetaZeroKind::InitiallyNonPositive,
            at: None,
            days_from_start: None,
            days_to_end: None,
        });
    }
    let ending = theta_at(&end_time);
    let Some(ending) = ending.filter(|theta| theta.is_finite()) else {
        return Ok(ThetaZeroSection {
            kind: ThetaZeroKind::Unavailable,
            at: None,
            days_from_start: None,
            days_to_end: None,
        });
    };
    if ending >= -ZERO_THETA {
        return Ok(ThetaZeroSection {
            kind: ThetaZeroKind::NoCrossing,
            at: None,
            days_from_start: None,
            days_to_end: None,
        });
    }

    let mut low = start_unix;
    let mut high = end_unix;
    let mut iterations = 0;
    while high - low > precision {
        if iterations >= max_iterations {
            return Ok(ThetaZeroSection {
                kind: ThetaZeroKind::BudgetExhausted,
                at: None,
                days_from_start: None,
                days_to_end: None,
            });
        }
        iterations += 1;
        let mid = low + (high - low) / 2;
        let mid_time = clock::from_unix_seconds(mid);
        let theta = theta_at(&mid_time);
        let Some(theta) = theta.filter(|value| value.is_finite()) else {
            return Ok(ThetaZeroSection {
                kind: ThetaZeroKind::Unavailable,
                at: None,
                days_from_start: None,
                days_to_end: None,
            });
        };
        if theta > ZERO_THETA {
            low = mid;
        } else {
            high = mid;
        }
    }
    let at = clock::from_unix_seconds(high);
    Ok(ThetaZeroSection {
        kind: ThetaZeroKind::Found,
        days_from_start: Some(
            clock::fractional_days_between(&start_time, &at)
                .map_err(|error| invalid(format!("theta zero day count failed: {error}")))?,
        ),
        days_to_end: Some(
            clock::fractional_days_between(&at, &end_time)
                .map_err(|error| invalid(format!("theta zero day count failed: {error}")))?,
        ),
        at: Some(at),
    })
}
