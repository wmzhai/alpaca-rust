use std::env::VarError;

use url::Url;

use crate::{Credentials, Error, validate};

pub const DEFAULT_API_KEY_ENV: &str = "APCA_API_KEY_ID";
pub const DEFAULT_SECRET_KEY_ENV: &str = "APCA_API_SECRET_KEY";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseUrl(String);

impl BaseUrl {
    pub fn new(value: impl AsRef<str>) -> Result<Self, Error> {
        let value = validate::non_empty_string("base_url", value.as_ref().to_owned())?;
        let parsed = Url::parse(&value).map_err(|error| {
            Error::InvalidConfiguration(format!("base_url must be a valid absolute URL: {error}"))
        })?;

        if parsed.scheme().is_empty() || parsed.host_str().is_none() {
            return Err(Error::InvalidConfiguration(
                "base_url must be a valid absolute URL".to_owned(),
            ));
        }

        Ok(Self(value.trim_end_matches('/').to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn join_path(&self, path: &str) -> String {
        format!("{}/{}", self.0, path.trim_start_matches('/'))
    }
}

pub fn credentials_from_env() -> Result<Option<Credentials>, Error> {
    credentials_from_env_names(DEFAULT_API_KEY_ENV, DEFAULT_SECRET_KEY_ENV)
}

pub fn credentials_from_env_names(
    api_key_var: &str,
    secret_key_var: &str,
) -> Result<Option<Credentials>, Error> {
    validate::valid_env_name("api_key_var", api_key_var)?;
    validate::valid_env_name("secret_key_var", secret_key_var)?;

    let api_key = read_var(api_key_var)?;
    let secret_key = read_var(secret_key_var)?;

    match (api_key, secret_key) {
        (Some(api_key), Some(secret_key)) => Credentials::new(api_key, secret_key).map(Some),
        (None, None) => Ok(None),
        _ => Err(Error::InvalidConfiguration(format!(
            "{api_key_var} and {secret_key_var} must be paired"
        ))),
    }
}

pub fn base_url_from_env_name(name: &str) -> Result<Option<BaseUrl>, Error> {
    validate::valid_env_name("base_url_var", name)?;

    match read_var(name)? {
        Some(value) => BaseUrl::new(value).map(Some),
        None => Ok(None),
    }
}

fn read_var(name: &str) -> Result<Option<String>, Error> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(Error::InvalidConfiguration(format!(
            "{name} must contain valid unicode"
        ))),
    }
}
