use std::fmt;

#[derive(Debug)]
pub enum SupportError {
    InvalidConfiguration(String),
    Core(alpaca_core::Error),
    Http(alpaca_http::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for SupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => write!(f, "invalid configuration: {message}"),
            Self::Core(error) => write!(f, "{error}"),
            Self::Http(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SupportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidConfiguration(_) => None,
            Self::Core(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

impl From<alpaca_core::Error> for SupportError {
    fn from(error: alpaca_core::Error) -> Self {
        Self::Core(error)
    }
}

impl From<alpaca_http::Error> for SupportError {
    fn from(error: alpaca_http::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for SupportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SupportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
