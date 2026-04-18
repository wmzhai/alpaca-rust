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
        self.extra_text("date")
    }

    #[must_use]
    pub fn occurred_at(&self) -> Option<&str> {
        self.date().or_else(|| self.sort_timestamp())
    }

    #[must_use]
    pub fn created_at(&self) -> Option<&str> {
        self.extra_text("created_at")
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.extra_text("description")
    }

    #[must_use]
    pub fn activity_sub_type(&self) -> Option<&str> {
        self.extra_text("activity_sub_type")
    }

    #[must_use]
    pub fn net_amount(&self) -> Option<Decimal> {
        self.extra_decimal("net_amount")
    }

    #[must_use]
    pub fn per_share_amount(&self) -> Option<Decimal> {
        self.extra_decimal("per_share_amount")
    }

    #[must_use]
    pub fn execution_id(&self) -> Option<&str> {
        self.extra_text("execution_id")
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

    fn extra_text(&self, key: &str) -> Option<&str> {
        self.extra.get(key).and_then(|value| value.as_str())
    }

    fn extra_decimal(&self, key: &str) -> Option<Decimal> {
        self.extra_text(key)
            .and_then(|value| value.parse::<Decimal>().ok())
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::{Activity, OptionActivityRecords};

    #[test]
    fn reads_common_activity_fields_from_flattened_extra_map() {
        let activity: Activity = serde_json::from_value(serde_json::json!({
            "id": "a1",
            "activity_type": "DIV",
            "transaction_time": null,
            "symbol": "SGOV",
            "qty": "5",
            "date": "2026-04-14",
            "created_at": "2026-04-14T11:30:00Z",
            "description": "Cash Dividend",
            "activity_sub_type": "OCC",
            "execution_id": "exec-1",
            "net_amount": "12.34",
            "per_share_amount": "2.468"
        }))
        .expect("activity should deserialize");

        assert_eq!(activity.date(), Some("2026-04-14"));
        assert_eq!(activity.occurred_at(), Some("2026-04-14"));
        assert_eq!(activity.created_at(), Some("2026-04-14T11:30:00Z"));
        assert_eq!(activity.description(), Some("Cash Dividend"));
        assert_eq!(activity.activity_sub_type(), Some("OCC"));
        assert_eq!(activity.execution_id(), Some("exec-1"));
        assert_eq!(activity.net_amount(), Some(Decimal::new(1234, 2)));
        assert_eq!(activity.per_share_amount(), Some(Decimal::new(2468, 3)));
        assert_eq!(activity.qty_i32(), Some(5));
        assert_eq!(activity.sort_timestamp(), Some("2026-04-14T11:30:00Z"));
    }

    #[test]
    fn prefers_transaction_time_for_sort_timestamp() {
        let activity: Activity = serde_json::from_value(serde_json::json!({
            "id": "a2",
            "activity_type": "FILL",
            "transaction_time": "2026-04-14T13:00:00Z",
            "created_at": "2026-04-14T11:30:00Z"
        }))
        .expect("activity should deserialize");

        assert_eq!(activity.sort_timestamp(), Some("2026-04-14T13:00:00Z"));
        assert_eq!(activity.occurred_at(), Some("2026-04-14T13:00:00Z"));
    }

    #[test]
    fn groups_option_assignment_and_expiration_records() {
        let assignment: Activity = serde_json::from_value(serde_json::json!({
            "id": "a3",
            "activity_type": "OPASN"
        }))
        .expect("assignment should deserialize");
        let expiration: Activity = serde_json::from_value(serde_json::json!({
            "id": "a4",
            "activity_type": "OPEXP"
        }))
        .expect("expiration should deserialize");
        let fill: Activity = serde_json::from_value(serde_json::json!({
            "id": "a5",
            "activity_type": "FILL"
        }))
        .expect("fill should deserialize");

        let grouped = OptionActivityRecords::from_activities(vec![
            assignment.clone(),
            fill,
            expiration.clone(),
        ]);

        assert_eq!(grouped.assignments, vec![assignment]);
        assert_eq!(grouped.expirations, vec![expiration]);
    }
}
