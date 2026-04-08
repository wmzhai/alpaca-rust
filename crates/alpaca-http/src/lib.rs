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
pub use observer::{ErrorEvent, NoopObserver, RequestStart, ResponseEvent, RetryEvent, TransportObserver};
pub use rate_limit::{ConcurrencyLimit, ConcurrencyPermit};
pub use request::{NoContent, RequestBody, RequestParts};
pub use retry::{RetryConfig, RetryDecision};
