use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheStats {
    pub subscribed_symbols: usize,
    pub subscribed_contracts: usize,
    pub subscribed_bar_requests: usize,
    pub cached_stocks: usize,
    pub cached_options: usize,
    pub cached_bar_symbols: usize,
    pub stocks_updated_at: Option<String>,
    pub options_updated_at: Option<String>,
    pub bars_updated_at: HashMap<String, String>,
}
