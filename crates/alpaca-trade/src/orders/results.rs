use crate::Error;

use super::{Order, OrderStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderTerminalState {
    Filled,
    Failed,
    Canceled,
    Expired,
    Rejected,
}

impl OrderTerminalState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Filled => "filled",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
        }
    }
}

impl OrderStatus {
    #[must_use]
    pub fn terminal_state(self) -> Option<OrderTerminalState> {
        match self {
            Self::Filled => Some(OrderTerminalState::Filled),
            Self::Failed => Some(OrderTerminalState::Failed),
            Self::Canceled => Some(OrderTerminalState::Canceled),
            Self::Expired => Some(OrderTerminalState::Expired),
            Self::Rejected => Some(OrderTerminalState::Rejected),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_finished(self) -> bool {
        self.is_filled() || self.is_failed_terminal()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcomeKind {
    Canceled,
    PartiallyFilledBeforeCancelCompleted,
    FilledBeforeCancelCompleted,
    Failed,
    Expired,
    Rejected,
}

impl CancelOutcomeKind {
    pub fn from_terminal_state(state: OrderTerminalState) -> Self {
        match state {
            OrderTerminalState::Filled => Self::FilledBeforeCancelCompleted,
            OrderTerminalState::Failed => Self::Failed,
            OrderTerminalState::Canceled => Self::Canceled,
            OrderTerminalState::Expired => Self::Expired,
            OrderTerminalState::Rejected => Self::Rejected,
        }
    }

    pub fn from_order(order: &Order) -> Option<Self> {
        let terminal_state = order.status.terminal_state()?;
        if terminal_state == OrderTerminalState::Filled {
            return Some(Self::FilledBeforeCancelCompleted);
        }
        if order.has_fill_evidence() {
            return Some(Self::PartiallyFilledBeforeCancelCompleted);
        }
        Some(Self::from_terminal_state(terminal_state))
    }
}

#[derive(Debug, Clone)]
pub struct CancelOutcome<T> {
    pub kind: CancelOutcomeKind,
    pub order: T,
    pub recovered_after_request_error: bool,
}

impl<T> CancelOutcome<T> {
    pub fn has_fill(&self) -> bool {
        matches!(
            self.kind,
            CancelOutcomeKind::PartiallyFilledBeforeCancelCompleted
                | CancelOutcomeKind::FilledBeforeCancelCompleted
        )
    }

    pub fn terminal_state(&self) -> OrderTerminalState {
        match self.kind {
            CancelOutcomeKind::Canceled => OrderTerminalState::Canceled,
            CancelOutcomeKind::PartiallyFilledBeforeCancelCompleted => OrderTerminalState::Canceled,
            CancelOutcomeKind::FilledBeforeCancelCompleted => OrderTerminalState::Filled,
            CancelOutcomeKind::Failed => OrderTerminalState::Failed,
            CancelOutcomeKind::Expired => OrderTerminalState::Expired,
            CancelOutcomeKind::Rejected => OrderTerminalState::Rejected,
        }
    }

    pub fn into_order(self) -> T {
        self.order
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcomeKind {
    OldOrderFilledBeforeReplace,
    ReplacedNewOrderPending,
    ReplacedNewOrderFilled,
    ReplaceFailedOldOrderTerminal(OrderTerminalState),
    ReplaceFailedNewOrderTerminal(OrderTerminalState),
    ReplaceFailedUnknown,
}

impl UpdateOutcomeKind {
    pub fn from_new_order_status(status: &str) -> Option<Self> {
        match OrderStatus::parse(status)
            .ok()
            .and_then(OrderStatus::terminal_state)
        {
            Some(OrderTerminalState::Filled) => Some(Self::ReplacedNewOrderFilled),
            Some(state) => Some(Self::ReplaceFailedNewOrderTerminal(state)),
            None => OrderStatus::parse(status)
                .ok()
                .map(|_| Self::ReplacedNewOrderPending),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateOutcome<T> {
    pub kind: UpdateOutcomeKind,
    pub old_order: Option<T>,
    pub new_order: Option<T>,
    pub recovered_after_request_error: bool,
    pub failure_reason: Option<String>,
}

impl<T> UpdateOutcome<T> {
    pub fn effective_order(&self) -> Option<&T> {
        self.new_order.as_ref().or(self.old_order.as_ref())
    }

    pub fn into_effective_order(self) -> Result<T, Error> {
        let Self {
            kind,
            old_order,
            new_order,
            failure_reason,
            ..
        } = self;

        new_order.or(old_order).ok_or_else(|| {
            Error::InvalidRequest(format!(
                "update outcome {:?} does not contain a usable order: {:?}",
                kind, failure_reason
            ))
        })
    }

    pub fn is_filled(&self) -> bool {
        matches!(
            self.kind,
            UpdateOutcomeKind::OldOrderFilledBeforeReplace
                | UpdateOutcomeKind::ReplacedNewOrderFilled
        )
    }
}
