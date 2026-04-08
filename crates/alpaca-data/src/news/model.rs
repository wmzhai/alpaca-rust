use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: i64,
    pub headline: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub summary: String,
    pub content: String,
    pub url: Option<String>,
    #[serde(default)]
    pub images: Vec<NewsImage>,
    #[serde(default)]
    pub symbols: Vec<String>,
    pub source: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NewsImage {
    pub size: String,
    pub url: String,
}
