use alpaca_core::{Error, pagination::PaginatedResponse};
use serde::{Deserialize, Serialize};

use super::NewsItem;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ListResponse {
    #[serde(default)]
    pub news: Vec<NewsItem>,
    pub next_page_token: Option<String>,
}

impl PaginatedResponse for ListResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, mut next: Self) -> Result<(), Error> {
        self.news.append(&mut next.news);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}
