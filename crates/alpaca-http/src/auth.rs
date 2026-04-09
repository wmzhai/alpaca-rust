use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::Error;

pub trait Authenticator: Send + Sync {
    fn apply(&self, headers: &mut HeaderMap) -> Result<(), Error>;
}

#[derive(Debug, Clone, Default)]
pub struct StaticHeaderAuthenticator {
    headers: HeaderMap,
}

impl StaticHeaderAuthenticator {
    pub fn from_pairs<I, K, V>(pairs: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut headers = HeaderMap::new();

        for (name, value) in pairs {
            let name = HeaderName::from_bytes(name.as_ref().as_bytes())
                .map_err(|error| Error::Authentication(format!("invalid header name: {error}")))?;
            let value = HeaderValue::from_str(value.as_ref())
                .map_err(|error| Error::Authentication(format!("invalid header value: {error}")))?;
            headers.insert(name, value);
        }

        Ok(Self { headers })
    }

    pub fn apply(&self, headers: &mut HeaderMap) -> Result<(), Error> {
        <Self as Authenticator>::apply(self, headers)
    }
}

impl Authenticator for StaticHeaderAuthenticator {
    fn apply(&self, headers: &mut HeaderMap) -> Result<(), Error> {
        headers.extend(self.headers.clone());
        Ok(())
    }
}
