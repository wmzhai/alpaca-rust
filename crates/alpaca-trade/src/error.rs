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

    /// Alpaca PATCH replace returns 422 when qty/limit/time_in_force are unchanged.
    /// That is a no-op success, not an unknown broker failure.
    #[must_use]
    pub fn is_unchanged_order_parameters(&self) -> bool {
        let Some(meta) = self.meta() else {
            return false;
        };
        meta.status() == 422
            && meta
                .body_snippet()
                .is_some_and(|snippet| snippet.contains("order parameters are not changed"))
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alpaca_http::{ErrorMeta, ResponseMeta};
    use reqwest::StatusCode;
    use reqwest::header::{HeaderMap, HeaderName};

    use super::Error;

    fn http_status_error(status: u16, body: &str) -> Error {
        let meta = ResponseMeta::from_response_parts(
            Some("patchOrderByOrderId".to_string()),
            "https://api.alpaca.markets/v2/orders/x".to_string(),
            StatusCode::from_u16(status).expect("status"),
            &HeaderMap::new(),
            &HeaderName::from_static("x-request-id"),
            1,
            Duration::from_millis(12),
        );
        alpaca_http::Error::HttpStatus(ErrorMeta::from_response_meta(meta, body.to_string())).into()
    }

    #[test]
    fn unchanged_order_parameters_matches_alpaca_422() {
        let error = http_status_error(
            422,
            r#"{"code":42210000,"message":"order parameters are not changed"}"#,
        );
        assert!(error.is_unchanged_order_parameters());
    }

    #[test]
    fn other_422_is_not_unchanged_order_parameters() {
        let error = http_status_error(422, r#"{"code":42210000,"message":"qty must be positive"}"#);
        assert!(!error.is_unchanged_order_parameters());
    }
}
