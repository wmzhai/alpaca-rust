use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("missing credentials")]
    MissingCredentials,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Http(#[from] alpaca_http::Error),
}
