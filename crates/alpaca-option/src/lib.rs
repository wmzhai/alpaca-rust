#![forbid(unsafe_code)]

//! alpaca-option
//!
//! Provider-neutral Rust option core semantics and math.
//! The public API is defined by the crate-level specifications.

pub mod analysis;
pub mod chain;
pub mod contract;
pub mod display;
pub mod error;
pub mod execution_quote;
pub mod expiration_selection;
pub mod liquidity;
pub mod math;
pub mod numeric;
pub mod payoff;
pub mod pricing;
pub mod probability;
pub mod snapshot;
pub mod types;
pub mod url;

pub const DEFAULT_RISK_FREE_RATE: f64 = 0.0368;

pub use error::{OptionError, OptionResult};
pub use liquidity::{LiquidityBatchResponse, LiquidityData, LiquidityOptionData, LiquidityStats};
pub use types::{
    AssignmentRiskLevel, BlackScholesImpliedVolatilityInput, BlackScholesInput, ContractDisplay,
    ExecutionAction, ExecutionLeg, ExecutionLegInput, ExecutionQuoteRange, ExecutionSnapshot,
    Greeks, GreeksInput, MoneynessLabel, OptionChain, OptionChainRecord, OptionContract,
    OptionPosition, OptionQuote, OptionRight, OptionRightCode, OptionSnapshot, OptionStratLegInput,
    OptionStratStockInput, OptionStratUrlInput, OrderSide, ParsedOptionStratUrl, PayoffLegInput,
    PositionIntent, PositionSide, QuotedLeg, RollLegSelection, RollRequest, ScaledExecutionQuote,
    ScaledExecutionQuoteRange, ShortItmPosition, StrategyBreakEvenInput, StrategyLegInput,
    StrategyPnlInput, StrategyValuationPosition,
};
