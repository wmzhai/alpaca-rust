use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Clone, Debug)]
pub struct StocksClient {
    inner: Arc<ClientInner>,
}

impl StocksClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }
}
