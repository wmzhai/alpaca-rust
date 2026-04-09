use serde_json::json;

use super::{JsonProbeResponse, LiveHttpProbe, SampleRecorder, ServiceConfig, SupportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperClock {
    pub timestamp: String,
    pub is_open: bool,
    pub next_open: String,
    pub next_close: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperSessionState {
    pub clock: PaperClock,
    pub has_calendar_session: bool,
}

pub async fn fetch_paper_clock(
    probe: &LiveHttpProbe,
    service: &ServiceConfig,
    recorder: Option<&SampleRecorder>,
) -> Result<PaperClock, SupportError> {
    let response = probe
        .get_json(service, "/v2/clock", Vec::<(String, String)>::new())
        .await?;
    maybe_record_clock_sample(recorder, &response)?;

    let body = response.body();
    Ok(PaperClock {
        timestamp: required_string_field(body, "timestamp")?,
        is_open: body
            .get("is_open")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| {
                SupportError::InvalidConfiguration(
                    "paper clock response was missing is_open".to_owned(),
                )
            })?,
        next_open: required_string_field(body, "next_open")?,
        next_close: required_string_field(body, "next_close")?,
    })
}

pub async fn paper_market_session_state(
    probe: &LiveHttpProbe,
    service: &ServiceConfig,
    recorder: Option<&SampleRecorder>,
) -> Result<PaperSessionState, SupportError> {
    let clock = fetch_paper_clock(probe, service, recorder).await?;
    let trading_day = trading_day_from_timestamp(&clock.timestamp)?;
    let response = probe
        .get_json(
            service,
            "/v2/calendar",
            [("start", trading_day.clone()), ("end", trading_day)],
        )
        .await?;
    maybe_record_calendar_sample(recorder, &response)?;
    let calendar_days = response.body().as_array().ok_or_else(|| {
        SupportError::InvalidConfiguration("paper calendar response was not an array".to_owned())
    })?;

    Ok(PaperSessionState {
        clock,
        has_calendar_session: !calendar_days.is_empty(),
    })
}

#[must_use]
pub fn can_submit_live_paper_orders(state: &PaperSessionState) -> bool {
    state.clock.is_open && state.has_calendar_session
}

pub fn trading_day_from_timestamp(timestamp: &str) -> Result<String, SupportError> {
    timestamp
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .or_else(|| timestamp.get(..10).map(ToOwned::to_owned))
        .ok_or_else(|| {
            SupportError::InvalidConfiguration(format!(
                "timestamp {timestamp} did not contain a trading day"
            ))
        })
}

fn required_string_field(body: &serde_json::Value, field: &str) -> Result<String, SupportError> {
    body.get(field)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            SupportError::InvalidConfiguration(format!("paper clock response was missing {field}"))
        })
}

fn maybe_record_clock_sample(
    recorder: Option<&SampleRecorder>,
    response: &JsonProbeResponse,
) -> Result<(), SupportError> {
    if let Some(recorder) = recorder {
        let payload = json!({
            "request_id": response.meta().request_id(),
            "status": response.meta().status(),
            "body": response.body(),
        });
        let _ = recorder.record_json("paper-clock", "clock", &payload)?;
    }

    Ok(())
}

fn maybe_record_calendar_sample(
    recorder: Option<&SampleRecorder>,
    response: &JsonProbeResponse,
) -> Result<(), SupportError> {
    if let Some(recorder) = recorder {
        let payload = json!({
            "request_id": response.meta().request_id(),
            "status": response.meta().status(),
            "body": response.body(),
        });
        let _ = recorder.record_json("paper-calendar", "calendar", &payload)?;
    }

    Ok(())
}
