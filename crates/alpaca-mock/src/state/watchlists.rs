use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use alpaca_trade::assets::Asset;
use alpaca_trade::watchlists::{Watchlist, WatchlistSummary};

use super::{MockStateError, assets, now_string};

#[derive(Debug, Clone, Default)]
pub(super) struct WatchlistBook {
    entries: Vec<Watchlist>,
    sequence: u64,
}

impl WatchlistBook {
    pub(super) fn summaries(&self) -> Vec<WatchlistSummary> {
        self.entries
            .iter()
            .map(|watchlist| WatchlistSummary {
                id: watchlist.id.clone(),
                account_id: watchlist.account_id.clone(),
                created_at: watchlist.created_at.clone(),
                updated_at: watchlist.updated_at.clone(),
                name: watchlist.name.clone(),
            })
            .collect()
    }

    pub(super) fn create(
        &mut self,
        account_id: &str,
        name: String,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        self.ensure_unique_name(&name, None)?;
        let assets = resolve_assets(symbols.unwrap_or_default())?;
        let now = now_string();
        let watchlist = Watchlist {
            id: self.next_id(account_id),
            account_id: account_id.to_owned(),
            created_at: now.clone(),
            updated_at: now,
            name,
            assets: Some(assets),
        };
        self.entries.push(watchlist.clone());
        Ok(watchlist)
    }

    pub(super) fn get_by_id(&self, watchlist_id: &str) -> Result<Watchlist, MockStateError> {
        self.entries
            .iter()
            .find(|watchlist| watchlist.id == watchlist_id)
            .cloned()
            .ok_or_else(|| watchlist_not_found_by_id(watchlist_id))
    }

    pub(super) fn get_by_name(&self, name: &str) -> Result<Watchlist, MockStateError> {
        self.entries
            .iter()
            .find(|watchlist| watchlist.name == name)
            .cloned()
            .ok_or_else(|| watchlist_not_found_by_name(name))
    }

    pub(super) fn update_by_id(
        &mut self,
        watchlist_id: &str,
        name: Option<String>,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.id == watchlist_id)
            .ok_or_else(|| watchlist_not_found_by_id(watchlist_id))?;
        self.update(index, name, symbols)
    }

    pub(super) fn update_by_name(
        &mut self,
        current_name: &str,
        name: Option<String>,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.name == current_name)
            .ok_or_else(|| watchlist_not_found_by_name(current_name))?;
        self.update(index, name, symbols)
    }

    pub(super) fn delete_by_id(&mut self, watchlist_id: &str) -> Result<(), MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.id == watchlist_id)
            .ok_or_else(|| watchlist_not_found_by_id(watchlist_id))?;
        self.entries.remove(index);
        Ok(())
    }

    pub(super) fn delete_by_name(&mut self, name: &str) -> Result<(), MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.name == name)
            .ok_or_else(|| watchlist_not_found_by_name(name))?;
        self.entries.remove(index);
        Ok(())
    }

    pub(super) fn add_asset_by_id(
        &mut self,
        watchlist_id: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.id == watchlist_id)
            .ok_or_else(|| watchlist_not_found_by_id(watchlist_id))?;
        self.add_asset(index, symbol)
    }

    pub(super) fn add_asset_by_name(
        &mut self,
        name: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        let index = self
            .entries
            .iter()
            .position(|watchlist| watchlist.name == name)
            .ok_or_else(|| watchlist_not_found_by_name(name))?;
        self.add_asset(index, symbol)
    }

    pub(super) fn remove_asset_by_id(
        &mut self,
        watchlist_id: &str,
        symbol: &str,
    ) -> Result<Watchlist, MockStateError> {
        let watchlist = self
            .entries
            .iter_mut()
            .find(|watchlist| watchlist.id == watchlist_id)
            .ok_or_else(|| watchlist_not_found_by_id(watchlist_id))?;
        let assets = watchlist.assets.get_or_insert_default();
        let index = assets
            .iter()
            .position(|asset| asset.symbol.eq_ignore_ascii_case(symbol))
            .ok_or_else(|| {
                MockStateError::NotFound(format!(
                    "symbol {symbol} was not found in watchlist {watchlist_id}"
                ))
            })?;
        assets.remove(index);
        watchlist.updated_at = now_string();
        Ok(watchlist.clone())
    }

    fn update(
        &mut self,
        index: usize,
        name: Option<String>,
        symbols: Option<Vec<String>>,
    ) -> Result<Watchlist, MockStateError> {
        if let Some(name) = name.as_deref() {
            self.ensure_unique_name(name, Some(index))?;
        }
        let assets = symbols.map(resolve_assets).transpose()?;
        let watchlist = &mut self.entries[index];
        if let Some(name) = name {
            watchlist.name = name;
        }
        if let Some(assets) = assets {
            watchlist.assets = Some(assets);
        }
        watchlist.updated_at = now_string();
        Ok(watchlist.clone())
    }

    fn add_asset(&mut self, index: usize, symbol: &str) -> Result<Watchlist, MockStateError> {
        let asset = resolve_asset(symbol)?;
        let watchlist = &mut self.entries[index];
        let assets = watchlist.assets.get_or_insert_default();
        if assets
            .iter()
            .any(|existing| existing.symbol.eq_ignore_ascii_case(&asset.symbol))
        {
            return Err(MockStateError::Conflict(format!(
                "symbol {} already exists in watchlist {}",
                asset.symbol, watchlist.name
            )));
        }
        assets.push(asset);
        watchlist.updated_at = now_string();
        Ok(watchlist.clone())
    }

    fn ensure_unique_name(
        &self,
        name: &str,
        excluded_index: Option<usize>,
    ) -> Result<(), MockStateError> {
        if self
            .entries
            .iter()
            .enumerate()
            .any(|(index, watchlist)| Some(index) != excluded_index && watchlist.name == name)
        {
            return Err(MockStateError::Conflict(format!(
                "watchlist name {name} already exists"
            )));
        }
        Ok(())
    }

    fn next_id(&mut self, account_id: &str) -> String {
        self.sequence = self.sequence.wrapping_add(1);
        let mut hasher = DefaultHasher::new();
        account_id.hash(&mut hasher);
        self.sequence.hash(&mut hasher);
        let suffix = hasher.finish() & 0x0000_ffff_ffff_ffff;
        format!("00000000-0000-4000-8000-{suffix:012x}")
    }
}

fn resolve_assets(symbols: Vec<String>) -> Result<Vec<Asset>, MockStateError> {
    let mut seen = HashSet::new();
    symbols
        .into_iter()
        .map(|symbol| {
            let asset = resolve_asset(&symbol)?;
            if !seen.insert(asset.symbol.clone()) {
                return Err(MockStateError::Conflict(format!(
                    "symbol {} appears more than once",
                    asset.symbol
                )));
            }
            Ok(asset)
        })
        .collect()
}

fn resolve_asset(symbol: &str) -> Result<Asset, MockStateError> {
    assets::catalog()
        .into_iter()
        .find(|asset| asset.symbol.eq_ignore_ascii_case(symbol))
        .ok_or_else(|| MockStateError::NotFound(format!("asset {symbol} was not found")))
}

fn watchlist_not_found_by_id(watchlist_id: &str) -> MockStateError {
    MockStateError::NotFound(format!("watchlist {watchlist_id} was not found"))
}

fn watchlist_not_found_by_name(name: &str) -> MockStateError {
    MockStateError::NotFound(format!("watchlist named {name} was not found"))
}
