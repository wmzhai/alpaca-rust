//! Async Rust client for the Alpaca Trading HTTP API.
//!
//! The default builder targets Alpaca paper trading. Use `Client::builder().live()`
//! to select the live base URL, or `base_url_str(...)` for a custom endpoint.
//!
//! Environment variables:
//!
//! - `ALPACA_TRADE_API_KEY`
//! - `ALPACA_TRADE_SECRET_KEY`
//! - `ALPACA_TRADE_BASE_URL`
//!
//! ```no_run
//! use alpaca_trade::Client;
//!
//! let client = Client::builder()
//!     .credentials_from_env()?
//!     .base_url_from_env()?
//!     .build()?;
//! let _account = client.account();
//! # Ok::<(), alpaca_trade::Error>(())
//! ```
//!
//! For mock-backed lifecycle validation, see `alpaca-mock` and the workspace
//! docs site at <https://wmzhai.github.io/alpaca-rust/>.
//!
#![forbid(unsafe_code)]

mod client;
mod error;
mod pagination;

pub mod account;
pub mod account_configurations;
pub mod activities;
pub mod assets;
pub mod calendar;
pub mod clock;
pub mod options_contracts;
pub mod orders;
pub mod portfolio_history;
pub mod positions;
pub mod watchlists;

pub use client::{
    Client, ClientBuilder, DEFAULT_LIVE_BASE_URL, DEFAULT_PAPER_BASE_URL, TRADE_API_KEY_ENV,
    TRADE_BASE_URL_ENV, TRADE_SECRET_KEY_ENV,
};
pub use error::Error;

#[cfg(test)]
mod tests;
