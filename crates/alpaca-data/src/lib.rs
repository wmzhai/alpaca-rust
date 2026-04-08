#![forbid(unsafe_code)]

#[derive(Debug, Clone, Default)]
pub struct Client;

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder;

impl Client {
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder
    }
}
