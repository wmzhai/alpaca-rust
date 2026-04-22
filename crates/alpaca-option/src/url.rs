use crate::contract::{build_occ_symbol, normalize_underlying_symbol, parse_occ_symbol};
use crate::display::format_strike;
use crate::error::{OptionError, OptionResult};
use crate::types::{
    OptionStratLegInput, OptionStratStockInput, OptionStratUrlInput, OrderSide,
    ParsedOptionStratUrl, StrategyLegInput,
};

const OPTIONSTRAT_BUILD_PREFIX: &str = "/build/";

#[derive(Debug, Clone, PartialEq)]
struct OptionStratLegFragmentInput {
    pub occ_symbol: String,
    pub quantity: i32,
    pub premium_per_contract: Option<f64>,
}

fn resolve_leg_input(input: &OptionStratLegInput) -> Option<OptionStratLegFragmentInput> {
    let occ_symbol = if input.occ_symbol.is_empty() {
        build_occ_symbol(
            input.underlying_symbol.as_deref().unwrap_or_default(),
            input.expiration_date.as_deref().unwrap_or_default(),
            input.strike.unwrap_or_default(),
            input.option_right.as_deref().unwrap_or_default(),
        )?
    } else {
        input.occ_symbol.clone()
    };

    if input.quantity == 0
        || input
            .premium_per_contract
            .map(|value| !value.is_finite())
            .unwrap_or(false)
    {
        None
    } else {
        Some(OptionStratLegFragmentInput {
            occ_symbol,
            quantity: input.quantity,
            premium_per_contract: input.premium_per_contract,
        })
    }
}

fn strategy_leg_to_build_input(leg: &StrategyLegInput) -> OptionStratLegInput {
    OptionStratLegInput {
        occ_symbol: leg.contract.occ_symbol.clone(),
        quantity: if leg.order_side == OrderSide::Sell {
            -(leg.ratio_quantity as i32)
        } else {
            leg.ratio_quantity as i32
        },
        premium_per_contract: leg.premium_per_contract,
        ..Default::default()
    }
}

pub fn to_optionstrat_underlying_path(symbol: &str) -> String {
    symbol.trim().replace('.', "/").replace('/', "%2F")
}

pub fn from_optionstrat_underlying_path(path: &str) -> String {
    path.replace("%2F", "/")
        .replace("%2f", "/")
        .replace('/', ".")
}

pub fn build_optionstrat_leg_fragment(input: &OptionStratLegInput) -> Option<String> {
    let leg = resolve_leg_input(input)?;
    let contract = parse_occ_symbol(&leg.occ_symbol)?;
    let prefix = if leg.quantity < 0 { "-." } else { "." };
    let compact_contract = format!(
        "{}{}{}{}",
        contract.underlying_symbol,
        contract.expiration_date[2..4].to_string()
            + &contract.expiration_date[5..7]
            + &contract.expiration_date[8..10],
        contract.option_right.code(),
        format_strike(contract.strike)
    );

    let premium_suffix = leg
        .premium_per_contract
        .map(|premium| format!("@{:.2}", premium.abs()))
        .unwrap_or_default();

    Some(format!(
        "{}{}x{}{}",
        prefix,
        compact_contract,
        leg.quantity.abs(),
        premium_suffix
    ))
}

pub fn build_optionstrat_stock_fragment(input: &OptionStratStockInput) -> Option<String> {
    if input.quantity <= 0 || !input.cost_per_share.is_finite() {
        return None;
    }

    let symbol = normalize_underlying_symbol(&input.underlying_symbol);
    if symbol.is_empty() {
        return None;
    }

    Some(format!(
        "{}x{}@{:.2}",
        symbol, input.quantity, input.cost_per_share
    ))
}

pub fn build_optionstrat_url(input: &OptionStratUrlInput) -> Option<String> {
    let leg_fragments = input
        .legs
        .iter()
        .map(build_optionstrat_leg_fragment)
        .collect::<Option<Vec<_>>>()?;
    let stock_fragments = input
        .stocks
        .iter()
        .map(build_optionstrat_stock_fragment)
        .collect::<Option<Vec<_>>>()?;
    let fragments = leg_fragments
        .into_iter()
        .chain(stock_fragments)
        .collect::<Vec<_>>();

    if fragments.is_empty() {
        return None;
    }

    Some(format!(
        "https://optionstrat.com/build/custom/{}/{}",
        to_optionstrat_underlying_path(&input.underlying_display_symbol),
        fragments.join(",")
    ))
}

pub fn merge_optionstrat_urls(
    urls: &[Option<String>],
    underlying_display_symbol: Option<&str>,
) -> Option<String> {
    let mut resolved_underlying = underlying_display_symbol.map(|value| value.to_string());
    let mut legs: Vec<OptionStratLegInput> = Vec::new();

    for raw_url in urls.iter().flatten() {
        let Ok(parsed) = parse_optionstrat_url(raw_url) else {
            continue;
        };

        if resolved_underlying.is_none() {
            resolved_underlying = Some(parsed.underlying_display_symbol.clone());
        }

        if resolved_underlying.as_deref() != Some(parsed.underlying_display_symbol.as_str()) {
            continue;
        }

        let Ok(parsed_legs) = parse_optionstrat_leg_fragments(
            parsed.underlying_display_symbol.as_str(),
            &parsed.leg_fragments,
        ) else {
            continue;
        };

        legs.extend(parsed_legs.iter().map(strategy_leg_to_build_input));
    }

    let underlying_display_symbol = resolved_underlying?;
    if legs.is_empty() {
        return None;
    }

    build_optionstrat_url(&OptionStratUrlInput {
        underlying_display_symbol,
        legs,
        stocks: Vec::new(),
    })
}

pub fn parse_optionstrat_url(url: &str) -> OptionResult<ParsedOptionStratUrl> {
    let without_suffix = url.split(['?', '#']).next().unwrap_or(url);
    let marker_index = without_suffix
        .find(OPTIONSTRAT_BUILD_PREFIX)
        .ok_or_else(|| {
            OptionError::new(
                "invalid_optionstrat_url",
                format!("invalid optionstrat url: {url}"),
            )
        })?;
    let rest = &without_suffix[marker_index + OPTIONSTRAT_BUILD_PREFIX.len()..];

    let mut parts = rest.splitn(3, '/');
    let strategy = parts.next().unwrap_or_default();
    let underlying_path = parts.next().unwrap_or_default();
    let fragments = parts.next().unwrap_or_default();

    if strategy.is_empty() || underlying_path.is_empty() {
        return Err(OptionError::new(
            "invalid_optionstrat_url",
            format!("invalid optionstrat url: {url}"),
        ));
    }

    Ok(ParsedOptionStratUrl {
        underlying_display_symbol: from_optionstrat_underlying_path(underlying_path),
        leg_fragments: if fragments.is_empty() {
            Vec::new()
        } else {
            fragments
                .split(',')
                .map(|fragment| fragment.to_string())
                .collect()
        },
    })
}

fn parse_compact_contract(input: &str) -> OptionResult<(String, String, char, f64)> {
    for (idx, ch) in input.char_indices() {
        if idx < 7 || idx + 1 >= input.len() || !matches!(ch, 'C' | 'P') {
            continue;
        }

        let date_start = idx - 6;
        let underlying = &input[..date_start];
        let date = &input[date_start..idx];
        let strike = &input[idx + 1..];

        if underlying.is_empty()
            || underlying.len() > 6
            || !underlying
                .chars()
                .all(|value| value.is_ascii_alphanumeric())
            || !date.chars().all(|value| value.is_ascii_digit())
            || strike.is_empty()
            || !strike
                .chars()
                .all(|value| value.is_ascii_digit() || value == '.')
        {
            continue;
        }

        let strike_value = strike.parse::<f64>().map_err(|_| {
            OptionError::new(
                "invalid_optionstrat_leg_fragment",
                format!("invalid compact contract: {input}"),
            )
        })?;
        let expiration_date = format!("20{}-{}-{}", &date[0..2], &date[2..4], &date[4..6]);
        return Ok((underlying.to_string(), expiration_date, ch, strike_value));
    }

    Err(OptionError::new(
        "invalid_optionstrat_leg_fragment",
        format!("invalid compact contract: {input}"),
    ))
}

fn parse_optionstrat_leg_fragment(
    fragment: &str,
    expected_underlying: &str,
) -> OptionResult<StrategyLegInput> {
    let (order_side, compact_fragment) = if let Some(value) = fragment.strip_prefix("-.") {
        (OrderSide::Sell, value)
    } else if let Some(value) = fragment.strip_prefix('.') {
        (OrderSide::Buy, value)
    } else {
        return Err(OptionError::new(
            "invalid_optionstrat_leg_fragment",
            format!("invalid optionstrat leg fragment: {fragment}"),
        ));
    };

    let (body, premium_part) = match compact_fragment.split_once('@') {
        Some((body, premium)) => (body, Some(premium)),
        None => (compact_fragment, None),
    };
    let (compact_contract, ratio_quantity) = match body.rsplit_once('x') {
        Some((compact_contract, quantity_text)) => {
            let ratio_quantity = quantity_text.parse::<u32>().map_err(|_| {
                OptionError::new(
                    "invalid_optionstrat_leg_fragment",
                    format!("invalid optionstrat leg fragment: {fragment}"),
                )
            })?;
            (compact_contract, ratio_quantity)
        }
        None => (body, 1),
    };
    if ratio_quantity == 0 {
        return Err(OptionError::new(
            "invalid_optionstrat_leg_fragment",
            format!("invalid optionstrat leg fragment: {fragment}"),
        ));
    }

    let (underlying_symbol, expiration_date, option_right_code, strike) =
        parse_compact_contract(compact_contract)?;
    if normalize_underlying_symbol(expected_underlying) != underlying_symbol {
        return Err(OptionError::new(
            "invalid_optionstrat_leg_fragment",
            format!("fragment underlying does not match: {fragment}"),
        ));
    }

    let occ_symbol = build_occ_symbol(
        &underlying_symbol,
        &expiration_date,
        strike,
        if option_right_code == 'C' {
            "call"
        } else {
            "put"
        },
    )
    .ok_or_else(|| {
        OptionError::new(
            "invalid_optionstrat_leg_fragment",
            format!("invalid optionstrat leg fragment: {fragment}"),
        )
    })?;
    let premium_per_contract = premium_part
        .map(|premium| {
            premium.parse::<f64>().map_err(|_| {
                OptionError::new(
                    "invalid_optionstrat_leg_fragment",
                    format!("invalid optionstrat leg fragment: {fragment}"),
                )
            })
        })
        .transpose()?
        .map(f64::abs);

    Ok(StrategyLegInput {
        contract: parse_occ_symbol(&occ_symbol).ok_or_else(|| {
            OptionError::new(
                "invalid_optionstrat_leg_fragment",
                format!("invalid optionstrat leg fragment: {fragment}"),
            )
        })?,
        order_side,
        ratio_quantity,
        premium_per_contract,
    })
}

pub fn parse_optionstrat_leg_fragments(
    underlying_display_symbol: &str,
    leg_fragments: &[String],
) -> OptionResult<Vec<StrategyLegInput>> {
    leg_fragments
        .iter()
        .map(|fragment| parse_optionstrat_leg_fragment(fragment, underlying_display_symbol))
        .collect()
}
