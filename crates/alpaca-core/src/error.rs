use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}
