use alpaca_time::display;

use crate::types::{ContractDisplay, OptionContract, OptionPosition, OptionRight, OptionRightCode};

pub fn format_strike(strike: f64) -> String {
    let mut text = format!("{strike:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

pub fn position_strike(position: &OptionPosition) -> String {
    format_strike(position.contract_info().strike)
}

pub fn compact_contract(contract: &OptionContract, expiration_style: Option<&str>) -> String {
    contract_display(contract, expiration_style).compact
}

pub fn contract_display(contract: &OptionContract, expiration_style: Option<&str>) -> ContractDisplay {
    let strike = format_strike(contract.strike);
    let option_right_code = contract.option_right.code_string();
    let expiration = display::compact(&contract.expiration_date, expiration_style.unwrap_or("mm-dd"));

    ContractDisplay {
        strike: strike.clone(),
        expiration: expiration.clone(),
        compact: format!("{strike}{}@{expiration}", contract.option_right.code()),
        option_right_code,
    }
}

pub fn option_right_code(option_right: &OptionRight) -> OptionRightCode {
    option_right.code_string()
}
