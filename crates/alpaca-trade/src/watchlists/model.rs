use serde::{Deserialize, Serialize};

use crate::assets::Asset;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchlistSummary {
    pub id: String,
    pub account_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Watchlist {
    pub id: String,
    pub account_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}
