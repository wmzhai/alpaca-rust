use rust_decimal::Decimal;
use serde_json::json;

use super::{JsonProbeResponse, LiveHttpProbe, SampleRecorder, ServiceConfig, SupportError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionContractType {
    Call,
    Put,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayWindow {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedOptionContract {
    pub symbol: String,
    pub underlying_symbol: String,
    pub expiration_date: String,
    pub contract_type: OptionContractType,
    pub strike_price: Decimal,
    pub reference_timestamp: Option<String>,
}

pub async fn discover_option_contracts(
    probe: &LiveHttpProbe,
    service: &ServiceConfig,
    recorder: Option<&SampleRecorder>,
    underlying_symbol: &str,
    limit: usize,
) -> Result<Vec<ObservedOptionContract>, SupportError> {
    let response = probe
        .get_json(
            service,
            &format!("/v1beta1/options/snapshots/{underlying_symbol}"),
            [("limit", limit.max(1).to_string())],
        )
        .await?;

    maybe_record_snapshots_sample(recorder, underlying_symbol, &response)?;

    let snapshots = response
        .body()
        .get("snapshots")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            SupportError::InvalidConfiguration(
                "options snapshots response was missing the snapshots object".to_owned(),
            )
        })?;

    let mut contracts = snapshots
        .iter()
        .map(|(symbol, snapshot)| {
            let mut contract = parse_occ_option_symbol(symbol)?;
            contract.reference_timestamp = reference_timestamp(snapshot);
            Ok(contract)
        })
        .collect::<Result<Vec<_>, SupportError>>()?;
    contracts.sort_by(|left, right| left.symbol.cmp(&right.symbol));

    if contracts.is_empty() {
        return Err(SupportError::InvalidConfiguration(format!(
            "options snapshots returned no contracts for {underlying_symbol}"
        )));
    }

    Ok(contracts)
}

pub async fn discover_active_option_contract(
    probe: &LiveHttpProbe,
    service: &ServiceConfig,
    recorder: Option<&SampleRecorder>,
    underlying_symbol: &str,
    limit: usize,
) -> Result<ObservedOptionContract, SupportError> {
    let contracts = discover_option_contracts(probe, service, recorder, underlying_symbol, limit).await?;

    contracts
        .iter()
        .find(|contract| contract.reference_timestamp.is_some())
        .cloned()
        .or_else(|| contracts.into_iter().next())
        .ok_or_else(|| {
            SupportError::InvalidConfiguration(format!(
                "options snapshots returned no usable contracts for {underlying_symbol}"
            ))
        })
}

pub fn parse_occ_option_symbol(symbol: &str) -> Result<ObservedOptionContract, SupportError> {
    if symbol.len() <= 15 {
        return Err(SupportError::InvalidConfiguration(format!(
            "option symbol {symbol} is shorter than the OCC contract suffix"
        )));
    }

    let root_end = symbol.len() - 15;
    let underlying_symbol = &symbol[..root_end];
    let suffix = &symbol[root_end..];
    let expiration_date = format!(
        "20{}-{}-{}",
        &suffix[0..2],
        &suffix[2..4],
        &suffix[4..6]
    );
    let contract_type = match &suffix[6..7] {
        "C" => OptionContractType::Call,
        "P" => OptionContractType::Put,
        value => {
            return Err(SupportError::InvalidConfiguration(format!(
                "option symbol {symbol} contained an unknown contract type marker {value}"
            )))
        }
    };
    let strike_suffix = suffix[7..15].parse::<i64>().map_err(|error| {
        SupportError::InvalidConfiguration(format!(
            "option symbol {symbol} contained an invalid strike suffix: {error}"
        ))
    })?;

    Ok(ObservedOptionContract {
        symbol: symbol.to_owned(),
        underlying_symbol: underlying_symbol.to_owned(),
        expiration_date,
        contract_type,
        strike_price: Decimal::new(strike_suffix, 3),
        reference_timestamp: None,
    })
}

pub fn full_day_window_from_timestamp(timestamp: &str) -> Result<DayWindow, SupportError> {
    let trading_day = timestamp
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .or_else(|| timestamp.get(..10).map(ToOwned::to_owned))
        .ok_or_else(|| {
            SupportError::InvalidConfiguration(format!(
                "timestamp {timestamp} did not contain a trading day"
            ))
        })?;

    Ok(DayWindow {
        start: format!("{trading_day}T00:00:00Z"),
        end: format!("{trading_day}T23:59:59Z"),
    })
}

fn reference_timestamp(snapshot: &serde_json::Value) -> Option<String> {
    snapshot
        .get("latestTrade")
        .and_then(|value| value.get("t"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            snapshot
                .get("minuteBar")
                .and_then(|value| value.get("t"))
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| {
            snapshot
                .get("latestQuote")
                .and_then(|value| value.get("t"))
                .and_then(serde_json::Value::as_str)
        })
        .map(ToOwned::to_owned)
}

fn maybe_record_snapshots_sample(
    recorder: Option<&SampleRecorder>,
    underlying_symbol: &str,
    response: &JsonProbeResponse,
) -> Result<(), SupportError> {
    if let Some(recorder) = recorder {
        let payload = json!({
            "underlying_symbol": underlying_symbol,
            "request_id": response.meta().request_id(),
            "status": response.meta().status(),
            "body": response.body(),
        });
        let _ = recorder.record_json("options-snapshots", underlying_symbol, &payload)?;
    }

    Ok(())
}
