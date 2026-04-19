use crate::calendar::{hhmm_to_total_minutes, market_hours_for_date};
use crate::clock::{now, parts};
use crate::error::TimeResult;
use crate::types::{MarketSession, TimestampParts};

fn timestamp_parts(timestamp: &str) -> TimeResult<TimestampParts> {
    parts(Some(timestamp))
}

pub fn market_session_at(timestamp: &str) -> TimeResult<MarketSession> {
    let parts = timestamp_parts(timestamp)?;
    let hours = market_hours_for_date(&parts.date)?;

    if !hours.is_trading_date {
        return Ok(MarketSession::Closed);
    }

    let minutes = hhmm_to_total_minutes(&parts.hhmm_string)?;
    let premarket_open = hhmm_to_total_minutes(hours.premarket_open.as_deref().unwrap())?;
    let regular_open = hhmm_to_total_minutes(hours.regular_open.as_deref().unwrap())?;
    let regular_close = hhmm_to_total_minutes(hours.regular_close.as_deref().unwrap())?;
    let after_hours_close = hhmm_to_total_minutes(hours.after_hours_close.as_deref().unwrap())?;

    if minutes >= premarket_open && minutes < regular_open {
        Ok(MarketSession::Premarket)
    } else if minutes >= regular_open && minutes < regular_close {
        Ok(MarketSession::Regular)
    } else if minutes >= regular_close && minutes < after_hours_close {
        Ok(MarketSession::AfterHours)
    } else {
        Ok(MarketSession::Closed)
    }
}

pub fn is_premarket_at(timestamp: &str) -> bool {
    matches!(market_session_at(timestamp), Ok(MarketSession::Premarket))
}

pub fn is_regular_session_at(timestamp: &str) -> bool {
    matches!(market_session_at(timestamp), Ok(MarketSession::Regular))
}

pub fn is_after_hours_at(timestamp: &str) -> bool {
    matches!(market_session_at(timestamp), Ok(MarketSession::AfterHours))
}

pub fn is_in_window(timestamp: &str, start: &str, end: &str) -> bool {
    let Ok(parts) = timestamp_parts(timestamp) else {
        return false;
    };
    let Ok(current) = hhmm_to_total_minutes(&parts.hhmm_string) else {
        return false;
    };
    let Ok(start) = hhmm_to_total_minutes(start) else {
        return false;
    };
    let Ok(end) = hhmm_to_total_minutes(end) else {
        return false;
    };
    if end < start {
        return false;
    }
    current >= start && current < end
}

pub fn is_overnight_window(timestamp: &str) -> bool {
    timestamp_parts(timestamp)
        .map(|parts| parts.hour >= 20 || parts.hour < 4)
        .unwrap_or(false)
}

pub fn is_regular_session_now() -> bool {
    is_regular_session_at(&now())
}

pub fn is_overnight_now() -> bool {
    is_overnight_window(&now())
}

pub fn is_in_window_now(start: &str, end: &str) -> bool {
    is_in_window(&now(), start, end)
}
