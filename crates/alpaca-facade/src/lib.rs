#![forbid(unsafe_code)]

//! alpaca-facade
//!
//! High-level convenience facades built on top of the lower-level Alpaca
//! crates.

pub mod options;
pub mod data;

pub use options::*;
pub use data::*;

pub type FacadeResult<T> = Result<T, anyhow::Error>;
