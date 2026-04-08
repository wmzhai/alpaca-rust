#![forbid(unsafe_code)]

mod client;
mod error;
mod pagination;

pub mod corporate_actions;
pub mod news;
pub mod options;
pub mod stocks;

pub use client::{
    Client, ClientBuilder, DATA_API_KEY_ENV, DATA_BASE_URL_ENV, DATA_SECRET_KEY_ENV,
    DEFAULT_DATA_BASE_URL, LEGACY_DATA_BASE_URL_ENV,
};
pub use error::Error;

#[cfg(test)]
mod tests;
