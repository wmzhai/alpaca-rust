use thiserror::Error;

use crate::meta::ErrorMeta;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("authentication error: {0}")]
    Authentication(String),
    #[error("concurrency limit error: {0}")]
    ConcurrencyLimit(String),
    #[error("transport error: {message}")]
    Transport {
        message: String,
        meta: Option<ErrorMeta>,
    },
    #[error("deserialize error: {message}")]
    Deserialize {
        message: String,
        meta: Option<ErrorMeta>,
    },
    #[error("http status error")]
    HttpStatus(ErrorMeta),
    #[error("rate limited")]
    RateLimited(ErrorMeta),
}

impl Error {
    #[must_use]
    pub fn meta(&self) -> Option<&ErrorMeta> {
        match self {
            Self::Transport { meta, .. } | Self::Deserialize { meta, .. } => meta.as_ref(),
            Self::HttpStatus(meta) | Self::RateLimited(meta) => Some(meta),
            Self::InvalidRequest(_) | Self::Authentication(_) | Self::ConcurrencyLimit(_) => None,
        }
    }

    #[must_use]
    pub fn from_reqwest(error: reqwest::Error, meta: Option<ErrorMeta>) -> Self {
        Self::Transport {
            message: error.to_string(),
            meta,
        }
    }
}
