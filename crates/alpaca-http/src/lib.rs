//! Shared HTTP transport for the `alpaca-rust` workspace.
//!
//! `alpaca-rest-http` provides the reusable request pipeline used by the higher-level
//! SDK crates. It includes request construction, retry policy, response and
//! error metadata, observer hooks, and concurrency limiting.
//!
//! Most applications should use `alpaca-data` or `alpaca-trade` instead of
//! depending on this crate directly.
//!
//! ```rust
//! use alpaca_http::{HttpClient, RetryConfig};
//!
//! let client = HttpClient::builder()
//!     .retry_config(RetryConfig::default())
//!     .build()?;
//! let _ = client;
//! # Ok::<(), alpaca_http::Error>(())
//! ```
//!
#![forbid(unsafe_code)]

pub mod auth;
pub mod client;
pub mod error;
pub mod meta;
pub mod observer;
pub mod rate_limit;
pub mod request;
pub mod retry;

pub use auth::{Authenticator, StaticHeaderAuthenticator};
pub use client::{HttpClient, HttpClientBuilder};
pub use error::Error;
pub use meta::{ErrorMeta, HttpResponse, ResponseMeta};
pub use observer::{
    ErrorEvent, NoopObserver, RequestStart, ResponseEvent, RetryEvent, TransportObserver,
};
pub use rate_limit::{ConcurrencyLimit, ConcurrencyPermit};
pub use request::{NoContent, RequestBody, RequestParts};
pub use retry::{RetryConfig, RetryDecision};
