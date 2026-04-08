use std::fmt;
use std::sync::Arc;

use crate::client::ClientInner;

#[derive(Clone)]
pub struct NewsClient {
    inner: Arc<ClientInner>,
}

impl NewsClient {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn inner(&self) -> &Arc<ClientInner> {
        &self.inner
    }
}

impl fmt::Debug for NewsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewsClient")
            .field("base_url", self.inner.base_url())
            .finish()
    }
}
