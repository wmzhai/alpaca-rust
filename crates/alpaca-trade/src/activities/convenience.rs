use rust_decimal::{Decimal, prelude::ToPrimitive};

use super::Activity;

impl Activity {
    #[must_use]
    pub fn date(&self) -> Option<&str> {
        self.extra_text("date")
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
    pub fn sort_timestamp(&self) -> Option<&str> {
        self.transaction_time
            .as_deref()
            .or_else(|| self.created_at())
    }

    #[must_use]
    pub fn qty_i32(&self) -> Option<i32> {
        self.qty.and_then(|value| value.trunc().to_i32())
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

    use super::Activity;

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
            "net_amount": "12.34",
            "per_share_amount": "2.468"
        }))
        .expect("activity should deserialize");

        assert_eq!(activity.date(), Some("2026-04-14"));
        assert_eq!(activity.created_at(), Some("2026-04-14T11:30:00Z"));
        assert_eq!(activity.description(), Some("Cash Dividend"));
        assert_eq!(activity.activity_sub_type(), Some("OCC"));
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
    }
}
