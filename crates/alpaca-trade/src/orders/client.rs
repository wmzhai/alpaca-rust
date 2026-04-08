use std::fmt;
use std::sync::Arc;

use alpaca_http::{NoContent, RequestParts};
use reqwest::Method;

use crate::client::ClientInner;
use crate::{
    Error,
    orders::{CancelAllOrderResult, CreateRequest, ListRequest, Order, ReplaceRequest},
};

#[derive(Clone)]
pub struct OrdersClient {
    inner: Arc<ClientInner>,
}

impl OrdersClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Order>, Error> {
        let request = RequestParts::new(Method::GET, "/v2/orders")
            .with_operation("orders.list")
            .with_query(request.into_query()?);

        self.inner
            .send_json::<Vec<Order>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn create(&self, request: CreateRequest) -> Result<Order, Error> {
        let request = RequestParts::new(Method::POST, "/v2/orders")
            .with_operation("orders.create")
            .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Order>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn cancel_all(&self) -> Result<Vec<CancelAllOrderResult>, Error> {
        let request =
            RequestParts::new(Method::DELETE, "/v2/orders").with_operation("orders.cancel_all");

        self.inner
            .send_json::<Vec<CancelAllOrderResult>>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn get(&self, order_id: &str) -> Result<Order, Error> {
        let request = RequestParts::new(
            Method::GET,
            format!(
                "/v2/orders/{}",
                super::request::validate_order_id(order_id)?
            ),
        )
        .with_operation("orders.get");

        self.inner
            .send_json::<Order>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn replace(&self, order_id: &str, request: ReplaceRequest) -> Result<Order, Error> {
        let request = RequestParts::new(
            Method::PATCH,
            format!(
                "/v2/orders/{}",
                super::request::validate_order_id(order_id)?
            ),
        )
        .with_operation("orders.replace")
        .with_json_body(request.into_json()?);

        self.inner
            .send_json::<Order>(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn cancel(&self, order_id: &str) -> Result<NoContent, Error> {
        let request = RequestParts::new(
            Method::DELETE,
            format!(
                "/v2/orders/{}",
                super::request::validate_order_id(order_id)?
            ),
        )
        .with_operation("orders.cancel");

        self.inner
            .send_no_content(request)
            .await
            .map(|response| response.into_body())
    }

    pub async fn get_by_client_order_id(&self, client_order_id: &str) -> Result<Order, Error> {
        let request = RequestParts::new(Method::GET, "/v2/orders:by_client_order_id")
            .with_operation("orders.get_by_client_order_id")
            .with_query(vec![(
                "client_order_id".to_owned(),
                super::request::validate_client_order_id(client_order_id)?,
            )]);

        self.inner
            .send_json::<Order>(request)
            .await
            .map(|response| response.into_body())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for OrdersClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrdersClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
