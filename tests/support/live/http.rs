use alpaca_core::{BaseUrl, Credentials};
use alpaca_http::{HttpClient, RequestParts, ResponseMeta, StaticHeaderAuthenticator};
use reqwest::Method;
use serde_json::Value;

use super::{SupportError, TradeServiceConfig};

#[derive(Debug, Clone)]
pub struct JsonProbeResponse {
    body: Value,
    meta: ResponseMeta,
}

impl JsonProbeResponse {
    #[must_use]
    pub fn body(&self) -> &Value {
        &self.body
    }

    #[must_use]
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }

    #[must_use]
    pub fn into_parts(self) -> (Value, ResponseMeta) {
        (self.body, self.meta)
    }
}

#[derive(Clone)]
pub struct LiveHttpProbe {
    client: HttpClient,
}

impl LiveHttpProbe {
    pub fn new() -> Result<Self, SupportError> {
        Ok(Self::from_client(HttpClient::builder().build()?))
    }

    #[must_use]
    pub fn from_client(client: HttpClient) -> Self {
        Self { client }
    }

    pub async fn get_trade_json<I, K, V>(
        &self,
        service: &TradeServiceConfig,
        path: &str,
        query: I,
    ) -> Result<JsonProbeResponse, SupportError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        self.get_json_with_base_url(
            service.credentials(),
            service.base_url().clone(),
            path,
            query,
        )
        .await
    }

    async fn get_json_with_base_url<I, K, V>(
        &self,
        credentials: &Credentials,
        base_url: BaseUrl,
        path: &str,
        query: I,
    ) -> Result<JsonProbeResponse, SupportError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        let request = RequestParts::new(Method::GET, path).with_query(
            query
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<Vec<_>>(),
        );
        let auth = StaticHeaderAuthenticator::from_pairs([
            ("APCA-API-KEY-ID", credentials.api_key()),
            ("APCA-API-SECRET-KEY", credentials.secret_key()),
        ])?;
        let response = self
            .client
            .send_json::<Value>(&base_url, request, Some(&auth))
            .await?;
        let (body, meta) = response.into_parts();

        Ok(JsonProbeResponse { body, meta })
    }
}
