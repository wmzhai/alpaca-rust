use alpaca_core::QueryWriter;

use crate::{
    Error,
    assets::{AssetAttribute, AssetClass, AssetStatus, Exchange},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub status: Option<AssetStatus>,
    pub asset_class: Option<AssetClass>,
    pub exchange: Option<Exchange>,
    pub attributes: Option<Vec<AssetAttribute>>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        query.push_opt("status", self.status);
        query.push_opt("asset_class", self.asset_class);
        query.push_opt("exchange", self.exchange);
        if let Some(attributes) = self.attributes.filter(|values| !values.is_empty()) {
            query.push_csv("attributes", attributes);
        }
        Ok(query.finish())
    }
}

pub(crate) fn validate_symbol_or_asset_id(symbol_or_asset_id: &str) -> Result<String, Error> {
    let trimmed = symbol_or_asset_id.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(
            "symbol_or_asset_id must not be empty or whitespace-only".to_owned(),
        ));
    }
    if trimmed.contains('/') {
        return Err(Error::InvalidRequest(
            "symbol_or_asset_id must not contain `/`".to_owned(),
        ));
    }

    Ok(trimmed.to_owned())
}
