use rust_decimal::{Decimal, prelude::ToPrimitive};

use super::Activity;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionActivityRecords {
    pub assignments: Vec<Activity>,
    pub expirations: Vec<Activity>,
}

impl OptionActivityRecords {
    #[must_use]
    pub fn from_activities(records: Vec<Activity>) -> Self {
        let mut grouped = Self::default();

        for record in records {
            if record.is_option_assignment() {
                grouped.assignments.push(record);
            } else if record.is_option_expiration() {
                grouped.expirations.push(record);
            }
        }

        grouped
    }
}

impl Activity {
    #[must_use]
    pub fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    #[must_use]
    pub fn occurred_at(&self) -> Option<&str> {
        self.date().or_else(|| self.sort_timestamp())
    }

    #[must_use]
    pub fn created_at(&self) -> Option<&str> {
        self.created_at.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub fn activity_sub_type(&self) -> Option<&str> {
        self.activity_sub_type.as_deref()
    }

    #[must_use]
    pub fn net_amount(&self) -> Option<Decimal> {
        self.net_amount
    }

    #[must_use]
    pub fn per_share_amount(&self) -> Option<Decimal> {
        self.per_share_amount
    }

    #[must_use]
    pub fn execution_id(&self) -> Option<&str> {
        self.execution_id.as_deref()
    }

    #[must_use]
    pub fn sort_timestamp(&self) -> Option<&str> {
        self.transaction_time
            .as_deref()
            .or_else(|| self.created_at())
    }

    #[must_use]
    pub fn qty_i32(&self) -> Option<i32> {
        self.qty.and_then(|value| value.trunc().to_i32())
    }

    #[must_use]
    pub fn is_option_assignment(&self) -> bool {
        self.activity_type == "OPASN"
    }

    #[must_use]
    pub fn is_option_expiration(&self) -> bool {
        self.activity_type == "OPEXP"
    }
}
