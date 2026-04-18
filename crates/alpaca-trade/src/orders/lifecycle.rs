use std::time::Duration;

use tokio::time::sleep;

use crate::Error;

use super::{CreateRequest, Order, OrderStatus, OrdersClient, ReplaceRequest};

const DEFAULT_WAIT_ATTEMPTS: usize = 30;
const DEFAULT_BASE_WAIT_MS: u64 = 100;
const DEFAULT_MAX_WAIT_MS: u64 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitFor {
    Stable,
    Filled,
    Canceled,
    Exact(OrderStatus),
}

impl WaitFor {
    fn matches(self, status: OrderStatus) -> bool {
        match self {
            Self::Stable => status.is_stable(),
            Self::Filled => status == OrderStatus::Filled,
            Self::Canceled => status.is_cancel_complete(),
            Self::Exact(expected) => status == expected,
        }
    }

    fn follows_replacements(self) -> bool {
        !matches!(self, Self::Exact(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedOrder {
    pub order: Order,
    pub recovered_after_request_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReplaceResolution {
    NewOrder(ResolvedOrder),
    OriginalOrderTerminal(ResolvedOrder),
}

impl ReplaceResolution {
    #[must_use]
    pub fn order(&self) -> &Order {
        match self {
            Self::NewOrder(resolved) | Self::OriginalOrderTerminal(resolved) => &resolved.order,
        }
    }

    #[must_use]
    pub fn recovered_after_request_error(&self) -> bool {
        match self {
            Self::NewOrder(resolved) | Self::OriginalOrderTerminal(resolved) => {
                resolved.recovered_after_request_error
            }
        }
    }
}

impl OrdersClient {
    pub async fn create_resolved(
        &self,
        request: CreateRequest,
        target: WaitFor,
    ) -> Result<ResolvedOrder, Error> {
        let created = self.create(request).await?;
        Ok(ResolvedOrder {
            order: self.wait_for(&created.id, target).await?,
            recovered_after_request_error: false,
        })
    }

    pub async fn get_effective(&self, order_id: &str) -> Result<Order, Error> {
        let mut current = self.get(order_id).await?;

        loop {
            if current.status != OrderStatus::Replaced {
                return Ok(current);
            }

            let replacement_id = current.replaced_by.as_deref().ok_or_else(|| {
                Error::InvalidRequest(
                    "order status is replaced but replaced_by is missing".to_owned(),
                )
            })?;

            current = self.get(replacement_id).await?;
        }
    }

    pub async fn wait_for(&self, order_id: &str, target: WaitFor) -> Result<Order, Error> {
        for attempt in 1..=DEFAULT_WAIT_ATTEMPTS {
            let order = if target.follows_replacements() {
                self.get_effective(order_id).await?
            } else {
                self.get(order_id).await?
            };

            if target.matches(order.status) {
                return Ok(order);
            }

            if attempt < DEFAULT_WAIT_ATTEMPTS {
                sleep(wait_delay(attempt)).await;
            }
        }

        if target.follows_replacements() {
            self.get_effective(order_id).await
        } else {
            self.get(order_id).await
        }
    }

    pub async fn cancel_resolved(&self, order_id: &str) -> Result<ResolvedOrder, Error> {
        match self.cancel(order_id).await {
            Ok(_) => Ok(ResolvedOrder {
                order: self.wait_for(order_id, WaitFor::Canceled).await?,
                recovered_after_request_error: false,
            }),
            Err(error) => match self.recover_cancel(order_id).await? {
                Some(order) => Ok(ResolvedOrder {
                    order,
                    recovered_after_request_error: true,
                }),
                None => Err(error),
            },
        }
    }

    pub async fn replace_resolved(
        &self,
        order_id: &str,
        request: ReplaceRequest,
    ) -> Result<ReplaceResolution, Error> {
        match self.replace(order_id, request).await {
            Ok(order) => Ok(ReplaceResolution::NewOrder(ResolvedOrder {
                order: self.wait_for(&order.id, WaitFor::Stable).await?,
                recovered_after_request_error: false,
            })),
            Err(error) => match self.recover_replace(order_id).await? {
                Some(resolution) => Ok(resolution),
                None => Err(error),
            },
        }
    }

    async fn recover_cancel(&self, order_id: &str) -> Result<Option<Order>, Error> {
        match self.wait_for(order_id, WaitFor::Canceled).await {
            Ok(order) => Ok(Some(order)),
            Err(_) => Ok(None),
        }
    }

    async fn recover_replace(&self, order_id: &str) -> Result<Option<ReplaceResolution>, Error> {
        for attempt in 1..=DEFAULT_WAIT_ATTEMPTS {
            let order = self.get(order_id).await?;

            if order.status.is_replace_recovery_terminal() {
                return Ok(Some(ReplaceResolution::OriginalOrderTerminal(
                    ResolvedOrder {
                        order,
                        recovered_after_request_error: true,
                    },
                )));
            }

            if order.status == OrderStatus::Replaced {
                let replacement_id = order.replaced_by.as_deref().ok_or_else(|| {
                    Error::InvalidRequest(
                        "order status is replaced but replaced_by is missing".to_owned(),
                    )
                })?;

                let order = self.wait_for(replacement_id, WaitFor::Stable).await?;
                return Ok(Some(ReplaceResolution::NewOrder(ResolvedOrder {
                    order,
                    recovered_after_request_error: true,
                })));
            }

            if attempt < DEFAULT_WAIT_ATTEMPTS {
                sleep(wait_delay(attempt)).await;
            }
        }

        Ok(None)
    }
}

fn wait_delay(attempt: usize) -> Duration {
    Duration::from_millis((DEFAULT_BASE_WAIT_MS * attempt as u64).min(DEFAULT_MAX_WAIT_MS))
}
