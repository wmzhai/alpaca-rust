use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Clone, Debug)]
pub struct OptionsClient {
    inner: Arc<ClientInner>,
}

impl OptionsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }
}
