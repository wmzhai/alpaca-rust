use axum::Json;
use axum::http::{HeaderMap, StatusCode, header::HeaderName};
use axum::response::{IntoResponse, Response};
use serde_json::json;

const APCA_API_KEY_ID: HeaderName = HeaderName::from_static("apca-api-key-id");
const APCA_API_SECRET_KEY: HeaderName = HeaderName::from_static("apca-api-secret-key");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MockAuth {
    pub(crate) api_key: String,
    pub(crate) secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthenticatedAccount {
    pub(crate) api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MockHttpError {
    status: StatusCode,
    message: String,
}

impl MockHttpError {
    pub(crate) fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub(crate) fn with_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for MockHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "code": self.status.as_u16(),
                "message": self.message,
            })),
        )
            .into_response()
    }
}

pub(crate) fn extract_auth(headers: &HeaderMap) -> Result<MockAuth, MockHttpError> {
    Ok(MockAuth {
        api_key: required_header(headers, &APCA_API_KEY_ID)?,
        secret_key: required_header(headers, &APCA_API_SECRET_KEY)?,
    })
}

fn required_header(headers: &HeaderMap, name: &HeaderName) -> Result<String, MockHttpError> {
    let value = headers
        .get(name)
        .ok_or_else(|| MockHttpError::unauthorized(format!("missing {} header", name.as_str())))?;
    let value = value
        .to_str()
        .map_err(|_| MockHttpError::unauthorized(format!("invalid {} header", name.as_str())))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(MockHttpError::unauthorized(format!(
            "missing {} header",
            name.as_str()
        )));
    }

    Ok(value.to_owned())
}
