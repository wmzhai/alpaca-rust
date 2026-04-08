use alpaca_core::{Error as CoreError, pagination::PaginatedResponse};
use serde::{Deserialize, Serialize};

use super::CorporateActions;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ListResponse {
    #[serde(default)]
    pub corporate_actions: CorporateActions,
    pub next_page_token: Option<String>,
}

impl PaginatedResponse for ListResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), CoreError> {
        self.corporate_actions.merge(next.corporate_actions);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}
