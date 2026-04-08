use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Clone, Debug)]
pub struct NewsClient {
    inner: Arc<ClientInner>,
}

impl NewsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }
}
