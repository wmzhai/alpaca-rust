use alpaca_http::ErrorMeta;
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

impl Error {
    #[must_use]
    pub fn meta(&self) -> Option<&ErrorMeta> {
        match self {
            Self::Http(error) => error.meta(),
            Self::InvalidConfiguration(_) | Self::MissingCredentials | Self::InvalidRequest(_) => {
                None
            }
        }
    }
}

impl From<alpaca_core::Error> for Error {
    fn from(error: alpaca_core::Error) -> Self {
        match error {
            alpaca_core::Error::InvalidConfiguration(message) => {
                Self::InvalidConfiguration(message)
            }
            alpaca_core::Error::InvalidRequest(message) => Self::InvalidRequest(message),
        }
    }
}
