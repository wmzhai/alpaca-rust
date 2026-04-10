//! Shared primitives for the `alpaca-rust` workspace.
//!
//! This crate contains lightweight building blocks reused by `alpaca-data`,
//! `alpaca-trade`, and `alpaca-mock`, including credentials, base URLs,
//! query serialization helpers, pagination helpers, and serde helpers.
//!
//! Most applications should start with `alpaca-data` or `alpaca-trade`.
//!
//! ```rust
//! use alpaca_core::Credentials;
//!
//! let credentials = Credentials::new("key", "secret")?;
//! assert_eq!(credentials.api_key(), "key");
//! # Ok::<(), alpaca_core::Error>(())
//! ```
//!
#![forbid(unsafe_code)]

mod auth;
pub mod decimal;
pub mod env;
mod error;
pub mod integer;
pub mod pagination;
mod query;
pub mod validate;

pub use auth::Credentials;
pub use env::BaseUrl;
pub use error::Error;
pub use query::QueryWriter;
