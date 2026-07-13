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
        let recovery_request = request.clone();
        match self.create(request).await {
            Ok(created) => Ok(ResolvedOrder {
                order: self.wait_for(&created.id, target).await?,
                recovered_after_request_error: false,
            }),
            Err(error) => match self
                .recover_create_by_client_order_id(&recovery_request, target)
                .await?
            {
                Some(order) => Ok(ResolvedOrder {
                    order,
                    recovered_after_request_error: true,
                }),
                None => Err(error),
            },
        }
    }

    pub(crate) async fn recover_created_once(
        &self,
        request: &CreateRequest,
        target: WaitFor,
    ) -> Result<Option<ResolvedOrder>, Error> {
        let Some(order) = self.find_created_by_client_order_id(request).await? else {
            return Ok(None);
        };
        Ok(Some(ResolvedOrder {
            order: self.wait_for(&order.id, target).await?,
            recovered_after_request_error: true,
        }))
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

    async fn recover_create_by_client_order_id(
        &self,
        request: &CreateRequest,
        target: WaitFor,
    ) -> Result<Option<Order>, Error> {
        if request.client_order_id.is_none() {
            return Ok(None);
        }

        for attempt in 1..=DEFAULT_WAIT_ATTEMPTS {
            if let Some(order) = self.find_created_by_client_order_id(request).await? {
                return Ok(Some(self.wait_for(&order.id, target).await?));
            }
            if attempt < DEFAULT_WAIT_ATTEMPTS {
                sleep(wait_delay(attempt)).await;
            }
        }

        Ok(None)
    }

    async fn find_created_by_client_order_id(
        &self,
        request: &CreateRequest,
    ) -> Result<Option<Order>, Error> {
        let Some(client_order_id) = request.client_order_id.as_deref() else {
            return Ok(None);
        };
        match self.get_by_client_order_id(client_order_id).await {
            Ok(order) => {
                validate_recovered_create_shape(request, &order)?;
                Ok(Some(order))
            }
            Err(error) if error.meta().is_some_and(|meta| meta.status() == 404) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn validate_recovered_create_shape(request: &CreateRequest, order: &Order) -> Result<(), Error> {
    let mismatch = |field: &str| {
        Error::InvalidRequest(format!(
            "recovered order does not match create request: {field}"
        ))
    };

    if request.client_order_id.as_deref() != Some(order.client_order_id.as_str()) {
        return Err(mismatch("client_order_id"));
    }
    if request
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol != &order.symbol)
    {
        return Err(mismatch("symbol"));
    }
    if request.qty != order.qty {
        return Err(mismatch("qty"));
    }
    if request.side.is_some_and(|side| side != order.side) {
        return Err(mismatch("side"));
    }
    if request.r#type != Some(order.r#type) {
        return Err(mismatch("type"));
    }
    if request.time_in_force != Some(order.time_in_force) {
        return Err(mismatch("time_in_force"));
    }
    if request.limit_price != order.limit_price {
        return Err(mismatch("limit_price"));
    }
    if request.order_class != Some(order.order_class) {
        return Err(mismatch("order_class"));
    }
    if request
        .extended_hours
        .is_some_and(|extended_hours| extended_hours != order.extended_hours)
    {
        return Err(mismatch("extended_hours"));
    }
    if request
        .position_intent
        .is_some_and(|position_intent| Some(position_intent) != order.position_intent)
    {
        return Err(mismatch("position_intent"));
    }

    match (&request.legs, &order.legs) {
        (None, None) => {}
        (None, Some(legs)) if legs.is_empty() => {}
        (Some(expected), Some(actual)) if expected.len() == actual.len() => {
            let mut matched = vec![false; actual.len()];
            for expected_leg in expected {
                let Some((index, _)) = actual.iter().enumerate().find(|(index, actual_leg)| {
                    !matched[*index]
                        && actual_leg.symbol == expected_leg.symbol
                        && actual_leg.ratio_qty == Some(expected_leg.ratio_qty)
                        && expected_leg.side.is_none_or(|side| side == actual_leg.side)
                        && expected_leg.position_intent.is_none_or(|position_intent| {
                            Some(position_intent) == actual_leg.position_intent
                        })
                }) else {
                    return Err(mismatch("legs"));
                };
                matched[index] = true;
            }
        }
        _ => return Err(mismatch("legs")),
    }

    Ok(())
}

fn wait_delay(attempt: usize) -> Duration {
    Duration::from_millis((DEFAULT_BASE_WAIT_MS * attempt as u64).min(DEFAULT_MAX_WAIT_MS))
}
