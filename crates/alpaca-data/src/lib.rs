//! Async Rust client for the Alpaca Market Data HTTP API.
//!
//! The crate follows a mirror layer plus thin convenience layer design:
//! resource methods track the official Alpaca HTTP API closely, while stable
//! helpers such as pagination aggregators remain opt-in.
//!
//! Environment variables:
//!
//! - `ALPACA_DATA_API_KEY`
//! - `ALPACA_DATA_SECRET_KEY`
//!
//! ```no_run
//! use alpaca_data::Client;
//!
//! let client = Client::builder().credentials_from_env()?.build()?;
//! let _stocks = client.stocks();
//! # Ok::<(), alpaca_data::Error>(())
//! ```
//!
//! See the workspace docs site at <https://wmzhai.github.io/alpaca-rust/>.
//!
#![forbid(unsafe_code)]

extern crate self as alpaca_data;

mod client;
mod error;
mod pagination;
mod symbols;

pub mod cache;
pub mod corporate_actions;
pub mod news;
pub mod options;
pub mod stocks;

pub use client::{Client, ClientBuilder, DATA_API_KEY_ENV, DATA_SECRET_KEY_ENV};
pub use error::Error;

#[cfg(test)]
mod tests;
