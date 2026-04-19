use alpaca_core::float;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::contract;
use crate::error::{OptionError, OptionResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum OptionRight {
    Call,
    Put,
}

impl Default for OptionRight {
    fn default() -> Self {
        Self::Call
    }
}

impl OptionRight {
    pub fn from_str(input: &str) -> OptionResult<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "call" => Ok(Self::Call),
            "put" => Ok(Self::Put),
            "c" => Ok(Self::Call),
            "p" => Ok(Self::Put),
            _ => Err(OptionError::new(
                "invalid_option_right",
                format!("invalid option right: {input}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Put => "put",
        }
    }

    pub fn from_code(code: char) -> OptionResult<Self> {
        match code {
            'C' => Ok(Self::Call),
            'P' => Ok(Self::Put),
            _ => Err(OptionError::new(
                "invalid_option_right_code",
                format!("invalid option right code: {code}"),
            )),
        }
    }

    pub fn code(&self) -> char {
        match self {
            Self::Call => 'C',
            Self::Put => 'P',
        }
    }

    pub fn code_string(&self) -> OptionRightCode {
        match self {
            Self::Call => OptionRightCode::C,
            Self::Put => OptionRightCode::P,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionRightCode {
    C,
    P,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn from_str(input: &str) -> OptionResult<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            _ => Err(OptionError::new(
                "invalid_order_side",
                format!("invalid order side: {input}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

impl PositionSide {
    pub fn from_str(input: &str) -> OptionResult<Self> {
        match input {
            "long" => Ok(Self::Long),
            "short" => Ok(Self::Short),
            _ => Err(OptionError::new(
                "invalid_position_side",
                format!("invalid position side: {input}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionAction {
    Open,
    Close,
}

impl ExecutionAction {
    pub fn from_str(input: &str) -> OptionResult<Self> {
        match input {
            "open" => Ok(Self::Open),
            "close" => Ok(Self::Close),
            _ => Err(OptionError::new(
                "invalid_execution_quote_input",
                format!("invalid execution action: {input}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Close => "close",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionIntent {
    BuyToOpen,
    SellToOpen,
    BuyToClose,
    SellToClose,
}

impl PositionIntent {
    pub fn from_str(input: &str) -> OptionResult<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "buy_to_open" => Ok(Self::BuyToOpen),
            "sell_to_open" => Ok(Self::SellToOpen),
            "buy_to_close" => Ok(Self::BuyToClose),
            "sell_to_close" => Ok(Self::SellToClose),
            _ => Err(OptionError::new(
                "invalid_position_intent",
                format!("invalid position intent: {input}"),
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BuyToOpen => "buy_to_open",
            Self::SellToOpen => "sell_to_open",
            Self::BuyToClose => "buy_to_close",
            Self::SellToClose => "sell_to_close",
        }
    }

    pub fn is_close(&self) -> bool {
        matches!(self, Self::BuyToClose | Self::SellToClose)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoneynessLabel {
    Itm,
    Atm,
    Otm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentRiskLevel {
    Danger,
    Critical,
    High,
    Medium,
    Low,
    Safe,
}

impl AssignmentRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Danger => "danger",
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Safe => "safe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionContract {
    pub underlying_symbol: String,
    pub expiration_date: String,
    pub strike: f64,
    pub option_right: OptionRight,
    pub occ_symbol: String,
}

impl Default for OptionContract {
    fn default() -> Self {
        Self {
            underlying_symbol: String::new(),
            expiration_date: String::new(),
            strike: 0.0,
            option_right: OptionRight::default(),
            occ_symbol: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionQuote {
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub mark: Option<f64>,
    pub last: Option<f64>,
}

impl Default for OptionQuote {
    fn default() -> Self {
        Self {
            bid: None,
            ask: None,
            mark: None,
            last: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDisplay {
    pub strike: String,
    pub expiration: String,
    pub compact: String,
    pub option_right_code: OptionRightCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

impl Default for Greeks {
    fn default() -> Self {
        Self {
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlackScholesInput {
    pub spot: f64,
    pub strike: f64,
    pub years: f64,
    pub rate: f64,
    pub dividend_yield: f64,
    pub volatility: f64,
    pub option_right: OptionRight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlackScholesImpliedVolatilityInput {
    pub target_price: f64,
    pub spot: f64,
    pub strike: f64,
    pub years: f64,
    pub rate: f64,
    pub dividend_yield: f64,
    pub option_right: OptionRight,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub tolerance: Option<f64>,
    pub max_iterations: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionSnapshot {
    pub as_of: String,
    pub contract: OptionContract,
    pub quote: OptionQuote,
    pub greeks: Option<Greeks>,
    pub implied_volatility: Option<f64>,
    pub underlying_price: Option<f64>,
}

impl Default for OptionSnapshot {
    fn default() -> Self {
        Self {
            as_of: String::new(),
            contract: OptionContract::default(),
            quote: OptionQuote::default(),
            greeks: None,
            implied_volatility: None,
            underlying_price: None,
        }
    }
}

fn parse_snapshot_number(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let value = trimmed.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn format_snapshot_number(value: f64) -> String {
    float::round(value, 2).to_string()
}

fn normalized_quote_price(quote: &OptionQuote) -> f64 {
    if let Some(mark) = quote.mark.filter(|value| value.is_finite()) {
        return mark;
    }

    match (
        quote.bid.filter(|value| value.is_finite()),
        quote.ask.filter(|value| value.is_finite()),
    ) {
        (Some(bid), Some(ask)) => float::round((bid + ask) / 2.0, 12),
        (Some(bid), None) => bid,
        (None, Some(ask)) => ask,
        (None, None) => quote.last.filter(|value| value.is_finite()).unwrap_or(0.0),
    }
}

fn canonical_contract_or_fallback(occ_symbol: &str) -> OptionContract {
    let normalized = occ_symbol.trim().to_ascii_uppercase();
    contract::parse_occ_symbol(&normalized).unwrap_or(OptionContract {
        occ_symbol: normalized,
        ..OptionContract::default()
    })
}

impl OptionSnapshot {
    pub fn is_empty(&self) -> bool {
        self.as_of.trim().is_empty()
            && self.contract.occ_symbol.trim().is_empty()
            && self.quote == OptionQuote::default()
            && self.greeks.is_none()
            && self.implied_volatility.is_none()
            && self.underlying_price.is_none()
    }

    pub fn occ_symbol(&self) -> &str {
        &self.contract.occ_symbol
    }

    pub fn timestamp(&self) -> &str {
        &self.as_of
    }

    pub fn bid(&self) -> f64 {
        self.quote
            .bid
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn ask(&self) -> f64 {
        self.quote
            .ask
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn price(&self) -> f64 {
        normalized_quote_price(&self.quote)
    }

    pub fn iv(&self) -> f64 {
        self.implied_volatility
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn delta(&self) -> f64 {
        self.greeks_or_default().delta
    }

    pub fn gamma(&self) -> f64 {
        self.greeks_or_default().gamma
    }

    pub fn vega(&self) -> f64 {
        self.greeks_or_default().vega
    }

    pub fn theta(&self) -> f64 {
        self.greeks_or_default().theta
    }

    pub fn rho(&self) -> f64 {
        self.greeks_or_default().rho
    }

    pub fn underlying_price(&self) -> f64 {
        self.underlying_price
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    }

    pub fn greeks_or_default(&self) -> Greeks {
        self.greeks.clone().unwrap_or_default()
    }
}

fn deserialize_position_snapshot<'de, D>(deserializer: D) -> Result<OptionSnapshot, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<OptionSnapshot>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionPosition {
    pub contract: String,
    #[serde(default, deserialize_with = "deserialize_position_snapshot")]
    pub snapshot: OptionSnapshot,
    pub qty: i32,
    #[serde(with = "alpaca_core::decimal::price_string_contract")]
    #[ts(type = "string")]
    pub avg_cost: Decimal,
    pub leg_type: String,
}

fn position_side_from_qty_and_leg_type(qty: i32, leg_type: &str) -> PositionSide {
    if qty < 0 {
        PositionSide::Short
    } else if qty > 0 {
        PositionSide::Long
    } else if leg_type.trim().to_ascii_lowercase().starts_with("short") {
        PositionSide::Short
    } else {
        PositionSide::Long
    }
}

impl OptionPosition {
    pub fn occ_symbol(&self) -> &str {
        self.contract.trim()
    }

    pub fn qty(&self) -> i32 {
        self.qty
    }

    pub fn contract_info(&self) -> OptionContract {
        canonical_contract_or_fallback(&self.contract)
    }

    pub fn position_side(&self) -> PositionSide {
        position_side_from_qty_and_leg_type(self.qty, &self.leg_type())
    }

    pub fn quantity(&self) -> u32 {
        self.qty.unsigned_abs()
    }

    pub fn snapshot_ref(&self) -> Option<&OptionSnapshot> {
        (!self.snapshot.is_empty()).then_some(&self.snapshot)
    }

    pub fn avg_cost(&self) -> f64 {
        self.avg_cost.to_f64().unwrap_or(0.0)
    }

    pub fn leg_type(&self) -> String {
        if !self.leg_type.trim().is_empty() {
            return self.leg_type.trim().to_ascii_lowercase();
        }

        let contract = self.contract_info();
        format!(
            "{}{}",
            self.position_side().as_str(),
            contract.option_right.as_str()
        )
    }

    pub fn cost(&self) -> Decimal {
        self.avg_cost * Decimal::from(self.qty) * Decimal::from(100)
    }

    pub fn value(&self) -> Decimal {
        alpaca_core::decimal::from_f64(self.snapshot.price(), 2)
            * Decimal::from(self.qty)
            * Decimal::from(100)
    }

    pub fn marked_value(&self) -> Decimal {
        self.value()
    }
}

impl Default for OptionPosition {
    fn default() -> Self {
        Self {
            contract: String::new(),
            snapshot: OptionSnapshot::default(),
            qty: 0,
            avg_cost: Decimal::ZERO,
            leg_type: String::new(),
        }
    }
}

impl TryFrom<&OptionPosition> for StrategyValuationPosition {
    type Error = OptionError;

    fn try_from(value: &OptionPosition) -> Result<Self, Self::Error> {
        let contract = contract::parse_occ_symbol(value.occ_symbol()).ok_or_else(|| {
            OptionError::new(
                "invalid_occ_symbol",
                format!("invalid occ symbol: {}", value.occ_symbol()),
            )
        })?;

        Ok(Self {
            contract,
            quantity: value.qty,
            avg_entry_price: Some(value.avg_cost()),
            implied_volatility: value.snapshot_ref().map(|snapshot| snapshot.iv()),
            mark_price: value.snapshot_ref().map(|snapshot| snapshot.price()),
            reference_underlying_price: value
                .snapshot_ref()
                .map(|snapshot| snapshot.underlying_price()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortItmPosition {
    pub contract: OptionContract,
    pub quantity: u32,
    pub option_price: f64,
    pub intrinsic: f64,
    pub extrinsic: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyLegInput {
    pub contract: OptionContract,
    pub order_side: OrderSide,
    pub ratio_quantity: u32,
    pub premium_per_contract: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotedLeg {
    pub contract: OptionContract,
    pub order_side: OrderSide,
    pub ratio_quantity: u32,
    pub quote: OptionQuote,
    pub snapshot: Option<OptionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GreeksInput {
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub rho: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub contract: String,
    pub timestamp: String,
    pub bid: String,
    pub ask: String,
    pub price: String,
    pub greeks: Greeks,
    pub iv: f64,
}

impl From<ExecutionSnapshot> for OptionSnapshot {
    fn from(value: ExecutionSnapshot) -> Self {
        Self {
            as_of: value.timestamp.trim().to_string(),
            contract: canonical_contract_or_fallback(&value.contract),
            quote: OptionQuote {
                bid: parse_snapshot_number(&value.bid),
                ask: parse_snapshot_number(&value.ask),
                mark: parse_snapshot_number(&value.price),
                last: parse_snapshot_number(&value.price),
            },
            greeks: Some(value.greeks),
            implied_volatility: value.iv.is_finite().then_some(value.iv),
            underlying_price: None,
        }
    }
}

impl From<&ExecutionSnapshot> for OptionSnapshot {
    fn from(value: &ExecutionSnapshot) -> Self {
        Self::from(value.clone())
    }
}

impl From<&OptionSnapshot> for ExecutionSnapshot {
    fn from(value: &OptionSnapshot) -> Self {
        Self {
            contract: value.occ_symbol().to_string(),
            timestamp: value.timestamp().to_string(),
            bid: format_snapshot_number(value.bid()),
            ask: format_snapshot_number(value.ask()),
            price: format_snapshot_number(value.price()),
            greeks: value.greeks_or_default(),
            iv: value.iv(),
        }
    }
}

impl From<OptionSnapshot> for ExecutionSnapshot {
    fn from(value: OptionSnapshot) -> Self {
        Self::from(&value)
    }
}

impl From<&OptionSnapshot> for OptionChainRecord {
    fn from(value: &OptionSnapshot) -> Self {
        Self {
            as_of: value.timestamp().to_string(),
            underlying_symbol: value.contract.underlying_symbol.clone(),
            occ_symbol: value.occ_symbol().to_string(),
            expiration_date: value.contract.expiration_date.clone(),
            option_right: value.contract.option_right.clone(),
            strike: value.contract.strike,
            underlying_price: value.underlying_price.filter(|number| number.is_finite()),
            bid: value.quote.bid.filter(|number| number.is_finite()),
            ask: value.quote.ask.filter(|number| number.is_finite()),
            mark: value.quote.mark.filter(|number| number.is_finite()),
            last: value.quote.last.filter(|number| number.is_finite()),
            implied_volatility: value.implied_volatility.filter(|number| number.is_finite()),
            delta: value
                .greeks
                .as_ref()
                .map(|greeks| greeks.delta)
                .filter(|number| number.is_finite()),
            gamma: value
                .greeks
                .as_ref()
                .map(|greeks| greeks.gamma)
                .filter(|number| number.is_finite()),
            vega: value
                .greeks
                .as_ref()
                .map(|greeks| greeks.vega)
                .filter(|number| number.is_finite()),
            theta: value
                .greeks
                .as_ref()
                .map(|greeks| greeks.theta)
                .filter(|number| number.is_finite()),
            rho: value
                .greeks
                .as_ref()
                .map(|greeks| greeks.rho)
                .filter(|number| number.is_finite()),
        }
    }
}

impl From<OptionSnapshot> for OptionChainRecord {
    fn from(value: OptionSnapshot) -> Self {
        Self::from(&value)
    }
}

impl From<&OptionChainRecord> for OptionSnapshot {
    fn from(value: &OptionChainRecord) -> Self {
        Self {
            as_of: value.as_of.trim().to_string(),
            contract: canonical_contract_or_fallback(&value.occ_symbol),
            quote: OptionQuote {
                bid: value.bid.filter(|number| number.is_finite()),
                ask: value.ask.filter(|number| number.is_finite()),
                mark: value
                    .mark
                    .filter(|number| number.is_finite() && *number > 0.0),
                last: value
                    .last
                    .filter(|number| number.is_finite() && *number > 0.0),
            },
            greeks: Some(Greeks {
                delta: value
                    .delta
                    .filter(|number| number.is_finite())
                    .unwrap_or(0.0),
                gamma: value
                    .gamma
                    .filter(|number| number.is_finite())
                    .unwrap_or(0.0),
                vega: value
                    .vega
                    .filter(|number| number.is_finite())
                    .unwrap_or(0.0),
                theta: value
                    .theta
                    .filter(|number| number.is_finite())
                    .unwrap_or(0.0),
                rho: value.rho.filter(|number| number.is_finite()).unwrap_or(0.0),
            }),
            implied_volatility: value.implied_volatility.filter(|number| number.is_finite()),
            underlying_price: value.underlying_price.filter(|number| number.is_finite()),
        }
    }
}

impl From<OptionChainRecord> for OptionSnapshot {
    fn from(value: OptionChainRecord) -> Self {
        Self::from(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionLeg {
    pub symbol: String,
    pub ratio_qty: String,
    pub side: OrderSide,
    pub position_intent: PositionIntent,
    pub leg_type: String,
    pub snapshot: Option<ExecutionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollLegSelection {
    pub leg_type: String,
    pub quantity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollRequest {
    pub current_contract: String,
    pub leg_type: Option<String>,
    pub qty: u32,
    pub new_strike: Option<f64>,
    pub new_expiration: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionLegInput {
    pub action: ExecutionAction,
    pub leg_type: String,
    pub contract: String,
    pub quantity: Option<u32>,
    pub snapshot: Option<ExecutionSnapshot>,
    pub timestamp: Option<String>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub price: Option<f64>,
    pub spread_percent: Option<f64>,
    pub greeks: Option<GreeksInput>,
    pub iv: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionQuoteRange {
    pub best_price: f64,
    pub worst_price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScaledExecutionQuote {
    pub structure_quantity: u32,
    pub price: f64,
    pub total_price: f64,
    pub total_dollars: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScaledExecutionQuoteRange {
    pub structure_quantity: u32,
    pub per_structure: ExecutionQuoteRange,
    pub per_order: ExecutionQuoteRange,
    pub dollars: ExecutionQuoteRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedOptionStratUrl {
    pub underlying_display_symbol: String,
    pub leg_fragments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OptionStratLegInput {
    pub occ_symbol: String,
    pub underlying_symbol: Option<String>,
    pub expiration_date: Option<String>,
    pub strike: Option<f64>,
    pub option_right: Option<String>,
    pub quantity: i32,
    pub premium_per_contract: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OptionStratStockInput {
    pub underlying_symbol: String,
    pub quantity: i32,
    pub cost_per_share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OptionStratUrlInput {
    pub underlying_display_symbol: String,
    #[serde(default)]
    pub legs: Vec<OptionStratLegInput>,
    #[serde(default)]
    pub stocks: Vec<OptionStratStockInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionChain {
    pub underlying_symbol: String,
    pub as_of: String,
    pub snapshots: Vec<OptionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OptionChainRecord {
    pub as_of: String,
    pub underlying_symbol: String,
    pub occ_symbol: String,
    pub expiration_date: String,
    pub option_right: OptionRight,
    pub strike: f64,
    pub underlying_price: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub mark: Option<f64>,
    pub last: Option<f64>,
    pub implied_volatility: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub rho: Option<f64>,
}

impl OptionChainRecord {
    pub fn is_delta_valid(&self) -> bool {
        self.delta
            .map(|delta| {
                let abs_delta = delta.abs();
                abs_delta >= 0.05 && abs_delta <= 0.95
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayoffLegInput {
    pub option_right: OptionRight,
    pub position_side: PositionSide,
    pub strike: f64,
    pub premium: f64,
    pub quantity: u32,
}

impl PayoffLegInput {
    pub fn new(
        option_right: &str,
        position_side: &str,
        strike: f64,
        premium: f64,
        quantity: u32,
    ) -> OptionResult<Self> {
        if quantity == 0 {
            return Err(OptionError::new(
                "invalid_payoff_input",
                format!("quantity must be greater than zero: {quantity}"),
            ));
        }

        Ok(Self {
            option_right: OptionRight::from_str(option_right)?,
            position_side: PositionSide::from_str(position_side)?,
            strike,
            premium,
            quantity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyValuationPosition {
    pub contract: OptionContract,
    pub quantity: i32,
    pub avg_entry_price: Option<f64>,
    pub implied_volatility: Option<f64>,
    pub mark_price: Option<f64>,
    pub reference_underlying_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyPnlInput {
    pub positions: Vec<StrategyValuationPosition>,
    pub underlying_price: f64,
    pub evaluation_time: String,
    pub entry_cost: Option<f64>,
    pub rate: f64,
    pub dividend_yield: Option<f64>,
    pub long_volatility_shift: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyBreakEvenInput {
    pub positions: Vec<StrategyValuationPosition>,
    pub evaluation_time: String,
    pub entry_cost: Option<f64>,
    pub rate: f64,
    pub dividend_yield: Option<f64>,
    pub long_volatility_shift: Option<f64>,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub scan_step: Option<f64>,
    pub tolerance: Option<f64>,
    pub max_iterations: Option<usize>,
}
