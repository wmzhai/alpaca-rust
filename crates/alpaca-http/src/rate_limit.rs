use std::{fmt, sync::Arc};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::Error;

#[derive(Clone, Debug, Default)]
pub struct ConcurrencyLimit {
    semaphore: Option<Arc<Semaphore>>,
}

impl ConcurrencyLimit {
    #[must_use]
    pub fn new(max_in_flight: Option<usize>) -> Self {
        Self {
            semaphore: max_in_flight
                .filter(|value| *value > 0)
                .map(|value| Arc::new(Semaphore::new(value))),
        }
    }

    pub async fn acquire(&self) -> Result<ConcurrencyPermit, Error> {
        match &self.semaphore {
            Some(semaphore) => {
                let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                    Error::ConcurrencyLimit("concurrency limit is closed".to_owned())
                })?;
                Ok(ConcurrencyPermit {
                    permit: Some(permit),
                })
            }
            None => Ok(ConcurrencyPermit { permit: None }),
        }
    }
}

pub struct ConcurrencyPermit {
    permit: Option<OwnedSemaphorePermit>,
}

impl fmt::Debug for ConcurrencyPermit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConcurrencyPermit")
            .field("held", &self.permit.is_some())
            .finish()
    }
}
