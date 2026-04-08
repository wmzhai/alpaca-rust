use std::fmt;

use rust_decimal::Decimal;

use alpaca_core::{QueryWriter, pagination::PaginatedRequest};

use crate::Error;

use super::{ContractStatus, ContractStyle, ContractType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRequest {
    pub underlying_symbols: Option<Vec<String>>,
    pub show_deliverables: Option<bool>,
    pub status: Option<ContractStatus>,
    pub expiration_date: Option<String>,
    pub expiration_date_gte: Option<String>,
    pub expiration_date_lte: Option<String>,
    pub root_symbol: Option<String>,
    pub r#type: Option<ContractType>,
    pub style: Option<ContractStyle>,
    pub strike_price_gte: Option<Decimal>,
    pub strike_price_lte: Option<Decimal>,
    pub page_token: Option<String>,
    pub limit: Option<u32>,
    pub ppind: Option<bool>,
}

impl ListRequest {
    pub(crate) fn into_query(self) -> Result<Vec<(String, String)>, Error> {
        let mut query = QueryWriter::default();
        if let Some(underlying_symbols) = validate_symbols(self.underlying_symbols)? {
            query.push_csv("underlying_symbols", underlying_symbols);
        }
        query.push_opt("show_deliverables", self.show_deliverables);
        query.push_opt("status", self.status);
        query.push_opt(
            "expiration_date",
            validate_optional_text("expiration_date", self.expiration_date)?,
        );
        query.push_opt(
            "expiration_date_gte",
            validate_optional_text("expiration_date_gte", self.expiration_date_gte)?,
        );
        query.push_opt(
            "expiration_date_lte",
            validate_optional_text("expiration_date_lte", self.expiration_date_lte)?,
        );
        query.push_opt(
            "root_symbol",
            validate_optional_text("root_symbol", self.root_symbol)?,
        );
        query.push_opt("type", self.r#type);
        query.push_opt("style", self.style);
        query.push_opt("strike_price_gte", self.strike_price_gte);
        query.push_opt("strike_price_lte", self.strike_price_lte);
        query.push_opt(
            "page_token",
            validate_optional_text("page_token", self.page_token)?,
        );
        query.push_opt("limit", validate_limit(self.limit, 1, 10_000)?);
        query.push_opt("ppind", self.ppind);
        Ok(query.finish())
    }
}

impl PaginatedRequest for ListRequest {
    fn with_page_token(&self, page_token: Option<String>) -> Self {
        let mut next = self.clone();
        next.page_token = page_token;
        next
    }
}

pub(crate) fn validate_symbol_or_id(symbol_or_id: &str) -> Result<String, Error> {
    let trimmed = symbol_or_id.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(
            "symbol_or_id must not be empty or whitespace-only".to_owned(),
        ));
    }
    if trimmed.contains('/') {
        return Err(Error::InvalidRequest(
            "symbol_or_id must not contain `/`".to_owned(),
        ));
    }

    Ok(trimmed.to_owned())
}

fn validate_optional_text(name: &str, value: Option<String>) -> Result<Option<String>, Error> {
    value
        .map(|value| validate_required_text(name, &value))
        .transpose()
}

fn validate_symbols(values: Option<Vec<String>>) -> Result<Option<Vec<String>>, Error> {
    match values {
        None => Ok(None),
        Some(values) if values.is_empty() => Err(Error::InvalidRequest(
            "underlying_symbols must contain at least one symbol".to_owned(),
        )),
        Some(values) => values
            .into_iter()
            .map(|value| validate_required_text("underlying_symbols", &value))
            .collect::<Result<Vec<_>, Error>>()
            .map(Some),
    }
}

fn validate_required_text(name: &str, value: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidRequest(format!(
            "{name} must not be empty or whitespace-only"
        )));
    }

    Ok(trimmed.to_owned())
}

fn validate_limit(limit: Option<u32>, min: u32, max: u32) -> Result<Option<u32>, Error> {
    match limit {
        Some(limit) if !(min..=max).contains(&limit) => Err(Error::InvalidRequest(format!(
            "limit must be between {min} and {max}"
        ))),
        _ => Ok(limit),
    }
}

impl fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractStatus::Active => "active",
            ContractStatus::Inactive => "inactive",
        })
    }
}

impl fmt::Display for ContractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractType::Call => "call",
            ContractType::Put => "put",
        })
    }
}

impl fmt::Display for ContractStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ContractStyle::American => "american",
            ContractStyle::European => "european",
        })
    }
}
