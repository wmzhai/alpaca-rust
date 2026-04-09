#![forbid(unsafe_code)]

mod client;
mod error;

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
