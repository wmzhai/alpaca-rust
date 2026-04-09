mod client;
mod model;
mod request;

pub use client::WatchlistsClient;
pub use model::{Watchlist, WatchlistSummary};
pub use request::{AddAssetRequest, CreateRequest, UpdateRequest};
