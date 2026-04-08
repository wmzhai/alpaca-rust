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

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn exposes_builder_entrypoint() {
        let _builder = Client::builder();
    }
}
