use alpaca_time::clock;

use crate::error::{OptionError, OptionResult};
use crate::types::{OptionContract, OptionPosition, OptionRight};

const OCC_TAIL_LENGTH: usize = 15;
const MAX_UNDERLYING_LENGTH: usize = 6;

fn canonical_underlying_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn ensure_underlying_symbol(symbol: &str) -> OptionResult<String> {
    let trimmed = symbol.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '/')
    {
        return Err(OptionError::new(
            "invalid_underlying_symbol",
            format!("invalid underlying symbol: {symbol}"),
        ));
    }

    let normalized = canonical_underlying_symbol(symbol);
    if normalized.is_empty() || normalized.len() > MAX_UNDERLYING_LENGTH {
        return Err(OptionError::new(
            "invalid_underlying_symbol",
            format!("invalid underlying symbol: {symbol}"),
        ));
    }
    Ok(normalized)
}

fn normalize_option_right_text(option_right: &str) -> Option<&'static str> {
    match option_right.trim().to_ascii_lowercase().as_str() {
        "call" | "c" => Some("call"),
        "put" | "p" => Some("put"),
        _ => None,
    }
}

fn canonical_contract_from_option_contract(contract: &OptionContract) -> Option<OptionContract> {
    if let Some(parsed) = parse_occ_symbol(&contract.occ_symbol) {
        return Some(parsed);
    }

    let occ_symbol = build_occ_symbol(
        &contract.underlying_symbol,
        &contract.expiration_date,
        contract.strike,
        contract.option_right.as_str(),
    )?;
    parse_occ_symbol(&occ_symbol)
}

pub trait ContractLike {
    fn canonical_contract(&self) -> Option<OptionContract>;
}

impl<T: ContractLike + ?Sized> ContractLike for &T {
    fn canonical_contract(&self) -> Option<OptionContract> {
        (*self).canonical_contract()
    }
}

impl ContractLike for OptionContract {
    fn canonical_contract(&self) -> Option<OptionContract> {
        canonical_contract_from_option_contract(self)
    }
}

impl ContractLike for str {
    fn canonical_contract(&self) -> Option<OptionContract> {
        parse_occ_symbol(self)
    }
}

impl ContractLike for String {
    fn canonical_contract(&self) -> Option<OptionContract> {
        self.as_str().canonical_contract()
    }
}

impl ContractLike for OptionPosition {
    fn canonical_contract(&self) -> Option<OptionContract> {
        parse_occ_symbol(&self.contract)
    }
}

pub fn normalize_underlying_symbol(symbol: &str) -> String {
    canonical_underlying_symbol(symbol)
}

pub fn is_occ_symbol(occ_symbol: &str) -> bool {
    parse_occ_symbol(occ_symbol).is_some()
}

pub fn parse_occ_symbol(occ_symbol: &str) -> Option<OptionContract> {
    let normalized = occ_symbol.trim().to_ascii_uppercase();
    if normalized.len() <= OCC_TAIL_LENGTH {
        return None;
    }

    let split = normalized.len() - OCC_TAIL_LENGTH;
    let underlying_symbol = &normalized[..split];
    if underlying_symbol.is_empty()
        || underlying_symbol.len() > MAX_UNDERLYING_LENGTH
        || !underlying_symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }

    let yy = &normalized[split..split + 2];
    let mm = &normalized[split + 2..split + 4];
    let dd = &normalized[split + 4..split + 6];
    let expiration_date = clock::parse_date(&format!("20{yy}-{mm}-{dd}")).ok()?;

    let option_right = OptionRight::from_code(normalized.as_bytes()[split + 6] as char).ok()?;

    let strike_digits = &normalized[split + 7..];
    if strike_digits.len() != 8 || !strike_digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let strike = strike_digits.parse::<u32>().ok()? as f64 / 1000.0;

    Some(OptionContract {
        underlying_symbol: underlying_symbol.to_string(),
        expiration_date,
        strike,
        option_right,
        occ_symbol: normalized,
    })
}

pub fn build_occ_symbol(
    underlying_symbol: &str,
    expiration_date: &str,
    strike: f64,
    option_right: &str,
) -> Option<String> {
    let underlying_symbol = ensure_underlying_symbol(underlying_symbol).ok()?;
    let expiration_date = clock::parse_date(expiration_date).ok()?;
    let option_right = OptionRight::from_str(normalize_option_right_text(option_right)?).ok()?;

    if !strike.is_finite() || strike < 0.0 {
        return None;
    }

    let strike_thousandths = (strike * 1000.0).round();
    if !(0.0..=99_999_999.0).contains(&strike_thousandths) {
        return None;
    }
    let yymmdd =
        expiration_date[2..4].to_string() + &expiration_date[5..7] + &expiration_date[8..10];

    Some(format!(
        "{}{}{}{:08}",
        underlying_symbol,
        yymmdd,
        option_right.code(),
        strike_thousandths as u32
    ))
}

pub fn canonical_contract(input: &impl ContractLike) -> Option<OptionContract> {
    input.canonical_contract()
}
