#![forbid(unsafe_code)]

//! alpaca-time
//!
//! Rust utilities for New York time and the US trading calendar.
//! The public API is defined by the crate-level specifications.

pub mod calendar;
pub mod chrono;
pub mod clock;
pub mod display;
pub mod error;
pub mod expiration;
pub mod range;
pub mod session;
pub mod types;

pub use error::{TimeError, TimeResult};
pub use types::{
    DateRange, DayCountBasis, DurationParts, MarketHours, MarketSession, TimestampParts,
    TradingDayInfo,
    WeekdayCode,
};
