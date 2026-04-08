use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Clone, Debug)]
pub struct CorporateActionsClient {
    inner: Arc<ClientInner>,
}

impl CorporateActionsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }
}
