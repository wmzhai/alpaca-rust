use alpaca_trade::calendar::{
    Calendar, CalendarDay, CalendarMarket, CalendarTimezone, CalendarV3Response, Market,
};

pub(crate) fn legacy_catalog() -> Vec<Calendar> {
    vec![Calendar {
        date: "2026-07-13".to_owned(),
        open: "09:30".to_owned(),
        close: "16:00".to_owned(),
        session_open: "04:00".to_owned(),
        session_close: "20:00".to_owned(),
        settlement_date: "2026-07-14".to_owned(),
    }]
}

pub(crate) fn v3_calendar(
    market: Market,
    start: Option<chrono::NaiveDate>,
    end: Option<chrono::NaiveDate>,
    timezone: Option<CalendarTimezone>,
) -> CalendarV3Response {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 13).expect("mock date should be valid");
    let included = start.is_none_or(|start| date >= start) && end.is_none_or(|end| date <= end);
    let utc = timezone == Some(CalendarTimezone::Utc);
    let (pre_start, pre_end, core_start, core_end, post_start, post_end) = if utc {
        (
            "2026-07-13T08:00:00Z",
            "2026-07-13T13:30:00Z",
            "2026-07-13T13:30:00Z",
            "2026-07-13T20:00:00Z",
            "2026-07-13T20:00:00Z",
            "2026-07-14T00:00:00Z",
        )
    } else {
        (
            "2026-07-13T04:00:00-04:00",
            "2026-07-13T09:30:00-04:00",
            "2026-07-13T09:30:00-04:00",
            "2026-07-13T16:00:00-04:00",
            "2026-07-13T16:00:00-04:00",
            "2026-07-13T20:00:00-04:00",
        )
    };

    CalendarV3Response {
        market: CalendarMarket {
            mic: (market == Market::NYSE).then(|| "XNYS".to_owned()),
            bic: None,
            acronym: market.to_string(),
            name: if market == Market::NYSE {
                "New York Stock Exchange".to_owned()
            } else {
                market.to_string()
            },
            timezone: if market == Market::NYSE {
                "America/New_York".to_owned()
            } else {
                "UTC".to_owned()
            },
        },
        calendar: included
            .then(|| CalendarDay {
                date: "2026-07-13".to_owned(),
                pre_start: Some(pre_start.to_owned()),
                pre_end: Some(pre_end.to_owned()),
                lunch_start: None,
                lunch_end: None,
                core_start: core_start.to_owned(),
                core_end: core_end.to_owned(),
                post_start: Some(post_start.to_owned()),
                post_end: Some(post_end.to_owned()),
                settlement_date: Some("2026-07-14".to_owned()),
            })
            .into_iter()
            .collect(),
    }
}
