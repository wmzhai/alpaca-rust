use crate::error::{OptionError, OptionResult};
use crate::pricing;
use crate::pricing::greeks_black_scholes;
use crate::types::{
    AssignmentRiskLevel, BlackScholesInput, MoneynessLabel, OptionPosition, OptionRight,
    PositionSide, ShortItmPosition,
};

const CONTRACT_MULTIPLIER: f64 = 100.0;

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_analysis_input",
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn ensure_positive(name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_analysis_input",
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

fn parse_option_right(option_right: &str) -> OptionResult<OptionRight> {
    OptionRight::from_str(option_right).map_err(|_| {
        OptionError::new(
            "invalid_analysis_input",
            format!("invalid option right: {option_right}"),
        )
    })
}

pub fn annualized_premium_yield(premium: f64, capital_base: f64, years: f64) -> OptionResult<f64> {
    ensure_finite("premium", premium)?;
    ensure_positive("capital_base", capital_base)?;
    ensure_positive("years", years)?;
    Ok(premium / capital_base / years)
}

pub fn annualized_premium_yield_days(
    premium: f64,
    capital_base: f64,
    calendar_days: i64,
) -> OptionResult<f64> {
    ensure_positive("calendar_days", calendar_days as f64)?;
    annualized_premium_yield(premium, capital_base, calendar_days as f64 / 365.0)
}

pub fn calendar_forward_factor(
    short_iv: f64,
    long_iv: f64,
    short_years: f64,
    long_years: f64,
) -> OptionResult<f64> {
    ensure_positive("short_iv", short_iv)?;
    ensure_positive("long_iv", long_iv)?;
    ensure_positive("short_years", short_years)?;
    ensure_positive("long_years", long_years)?;
    if long_years <= short_years {
        return Err(OptionError::new(
            "invalid_analysis_input",
            format!("long_years must be greater than short_years: {long_years} <= {short_years}"),
        ));
    }

    let short_variance = short_iv * short_iv;
    let long_variance = long_iv * long_iv;
    let forward_variance =
        (long_variance * long_years - short_variance * short_years) / (long_years - short_years);
    if forward_variance <= 0.0 {
        return Err(OptionError::new(
            "invalid_analysis_input",
            format!("forward variance must be positive: {forward_variance}"),
        ));
    }

    let forward_iv = forward_variance.sqrt();
    Ok((short_iv - forward_iv) / forward_iv)
}

pub fn moneyness_ratio(spot: f64, strike: f64) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    Ok(spot / strike)
}

pub fn moneyness_label(
    spot: f64,
    strike: f64,
    option_right: &str,
    atm_band: Option<f64>,
) -> OptionResult<MoneynessLabel> {
    let option_right = parse_option_right(option_right)?;
    let atm_band = atm_band.unwrap_or(0.02);
    ensure_finite("atm_band", atm_band)?;
    if atm_band < 0.0 {
        return Err(OptionError::new(
            "invalid_analysis_input",
            format!("atm_band must be non-negative: {atm_band}"),
        ));
    }

    let ratio = moneyness_ratio(spot, strike)?;
    if (ratio - 1.0).abs() <= atm_band {
        return Ok(MoneynessLabel::Atm);
    }

    Ok(match option_right {
        OptionRight::Call => {
            if ratio > 1.0 {
                MoneynessLabel::Itm
            } else {
                MoneynessLabel::Otm
            }
        }
        OptionRight::Put => {
            if ratio < 1.0 {
                MoneynessLabel::Itm
            } else {
                MoneynessLabel::Otm
            }
        }
    })
}

pub fn otm_percent(spot: f64, strike: f64, option_right: &str) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    let option_right = parse_option_right(option_right)?;

    Ok(match option_right {
        OptionRight::Call => (strike - spot) / spot * 100.0,
        OptionRight::Put => (spot - strike) / spot * 100.0,
    })
}

pub fn position_otm_percent(spot: f64, position: &OptionPosition) -> OptionResult<f64> {
    let contract = position.contract_info();
    otm_percent(spot, contract.strike, contract.option_right.as_str())
}

pub fn assignment_risk(extrinsic: f64) -> OptionResult<AssignmentRiskLevel> {
    ensure_finite("extrinsic", extrinsic)?;

    Ok(if extrinsic < 0.0 {
        AssignmentRiskLevel::Danger
    } else if extrinsic < 0.05 {
        AssignmentRiskLevel::Critical
    } else if extrinsic < 0.1 {
        AssignmentRiskLevel::High
    } else if extrinsic < 0.3 {
        AssignmentRiskLevel::Medium
    } else if extrinsic < 1.0 {
        AssignmentRiskLevel::Low
    } else {
        AssignmentRiskLevel::Safe
    })
}

pub fn short_extrinsic_amount(
    spot: f64,
    positions: &[OptionPosition],
    structure_quantity: Option<u32>,
) -> OptionResult<Option<f64>> {
    if !spot.is_finite() || spot <= 0.0 {
        return Ok(None);
    }

    let mut total_extrinsic_per_share = 0.0;
    let mut has_short_position = false;

    for position in positions {
        if position.position_side() != PositionSide::Short {
            continue;
        }

        has_short_position = true;
        if position.quantity() == 0 {
            return Ok(None);
        }

        let option_price = position
            .snapshot_ref()
            .and_then(|snapshot| snapshot.quote.mark.or(snapshot.quote.last));
        let Some(option_price) = option_price else {
            return Ok(None);
        };
        let contract = position.contract_info();

        total_extrinsic_per_share += pricing::extrinsic_value(
            option_price,
            spot,
            contract.strike,
            contract.option_right.as_str(),
        )? * f64::from(position.quantity());
    }

    if !has_short_position {
        return Ok(None);
    }

    let structure_quantity = structure_quantity.unwrap_or(1).max(1);
    Ok(Some(
        total_extrinsic_per_share * CONTRACT_MULTIPLIER * f64::from(structure_quantity),
    ))
}

pub fn short_itm_positions(
    spot: f64,
    positions: &[OptionPosition],
) -> OptionResult<Vec<ShortItmPosition>> {
    if !spot.is_finite() || spot <= 0.0 {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for position in positions {
        if position.position_side() != PositionSide::Short || position.quantity() == 0 {
            continue;
        }

        let option_price = position
            .snapshot_ref()
            .and_then(|snapshot| snapshot.quote.mark.or(snapshot.quote.last))
            .unwrap_or(0.0);
        let contract = position.contract_info();
        let intrinsic =
            pricing::intrinsic_value(spot, contract.strike, contract.option_right.as_str())?;
        if intrinsic <= 0.0 {
            continue;
        }

        items.push(ShortItmPosition {
            contract,
            quantity: position.quantity(),
            option_price,
            intrinsic,
            extrinsic: pricing::extrinsic_value(
                option_price,
                spot,
                position.contract_info().strike,
                position.contract_info().option_right.as_str(),
            )?,
        });
    }

    Ok(items)
}

pub fn strike_for_target_delta(
    spot: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    target_delta: f64,
    option_right: &str,
    strike_step: f64,
) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_finite("dividend_yield", dividend_yield)?;
    ensure_positive("volatility", volatility)?;
    ensure_finite("target_delta", target_delta)?;
    ensure_positive("strike_step", strike_step)?;
    let option_right = parse_option_right(option_right)?;

    match option_right {
        OptionRight::Call => {
            let mut strike = (spot / strike_step).round() * strike_step;
            while strike < spot * 1.5 + strike_step * 0.5 {
                let greeks = greeks_black_scholes(&BlackScholesInput {
                    spot,
                    strike,
                    years,
                    rate,
                    dividend_yield,
                    volatility,
                    option_right: OptionRight::Call,
                })?;
                if greeks.delta <= target_delta {
                    return Ok(strike);
                }
                strike += strike_step;
            }
        }
        OptionRight::Put => {
            let mut strike = (spot * 0.7 / strike_step).round() * strike_step;
            while strike <= spot * 1.1 + strike_step * 0.5 {
                let greeks = greeks_black_scholes(&BlackScholesInput {
                    spot,
                    strike,
                    years,
                    rate,
                    dividend_yield,
                    volatility,
                    option_right: OptionRight::Put,
                })?;
                if greeks.delta <= target_delta {
                    return Ok(strike);
                }
                strike += strike_step;
            }
        }
    }

    Err(OptionError::new(
        "target_delta_not_found",
        format!("no strike found for target delta: {target_delta}"),
    ))
}
