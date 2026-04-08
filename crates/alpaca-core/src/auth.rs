use std::fmt;

use crate::{Error, validate};

#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    api_key: String,
    secret_key: String,
}

impl Credentials {
    pub fn new(api_key: impl Into<String>, secret_key: impl Into<String>) -> Result<Self, Error> {
        let api_key = validate::non_empty_string("api_key", api_key)?;
        let secret_key = validate::non_empty_string("secret_key", secret_key)?;
        validate::valid_header_value("api_key", &api_key)?;
        validate::valid_header_value("secret_key", &secret_key)?;

        Ok(Self {
            api_key,
            secret_key,
        })
    }

    #[must_use]
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    #[must_use]
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("api_key", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}
