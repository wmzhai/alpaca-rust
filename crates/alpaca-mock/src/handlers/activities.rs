use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use serde::Deserialize;

use alpaca_trade::{
    activities::{Activity, ActivityCategory},
    orders::SortDirection,
};

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::{ListActivitiesFilter, MockServerState};

#[derive(Debug, Deserialize, Default)]
pub(crate) struct ListActivitiesQuery {
    activity_types: Option<String>,
    category: Option<ActivityCategory>,
    date: Option<String>,
    until: Option<String>,
    after: Option<String>,
    direction: Option<SortDirection>,
    page_size: Option<u32>,
    page_token: Option<String>,
}

pub(crate) async fn activities_list(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Query(query): Query<ListActivitiesQuery>,
) -> Result<Json<Vec<Activity>>, MockHttpError> {
    Ok(Json(state.list_activities(
        &account.api_key,
        query.into_filter(None)?,
    )))
}

pub(crate) async fn activities_by_type(
    State(state): State<MockServerState>,
    Extension(account): Extension<AuthenticatedAccount>,
    Path(activity_type): Path<String>,
    Query(query): Query<ListActivitiesQuery>,
) -> Result<Json<Vec<Activity>>, MockHttpError> {
    Ok(Json(state.list_activities(
        &account.api_key,
        query.into_filter(Some(activity_type))?,
    )))
}

impl ListActivitiesQuery {
    fn into_filter(
        self,
        activity_type: Option<String>,
    ) -> Result<ListActivitiesFilter, MockHttpError> {
        if !matches!(self.page_size, None | Some(1..=100)) {
            return Err(MockHttpError::bad_request(
                "page_size must be between 1 and 100",
            ));
        }
        if activity_type.is_none() && self.activity_types.is_some() && self.category.is_some() {
            return Err(MockHttpError::bad_request(
                "activity_types and category are mutually exclusive",
            ));
        }
        if activity_type.is_some() && (self.activity_types.is_some() || self.category.is_some()) {
            return Err(MockHttpError::bad_request(
                "by-type activity requests do not accept activity_types or category",
            ));
        }
        if activity_type
            .as_deref()
            .is_some_and(|value| !is_canonical_activity_type(value))
        {
            return Err(MockHttpError::bad_request(
                "activity_type must be a canonical Alpaca activity type",
            ));
        }

        let activity_types = activity_type
            .map(|activity_type| vec![activity_type])
            .or_else(|| {
                self.activity_types.map(|activity_types| {
                    activity_types
                        .split(',')
                        .map(|activity_type| activity_type.trim().to_owned())
                        .filter(|activity_type| !activity_type.is_empty())
                        .collect::<Vec<_>>()
                })
            });
        if activity_types.as_ref().is_some_and(|values| {
            values.is_empty()
                || values
                    .iter()
                    .any(|value| !is_canonical_activity_type(value))
        }) {
            return Err(MockHttpError::bad_request(
                "activity_types must contain canonical Alpaca activity types",
            ));
        }

        Ok(ListActivitiesFilter {
            activity_types,
            category: self.category,
            date: self.date,
            until: self.until,
            after: self.after,
            direction: self.direction,
            page_size: self.page_size,
            page_token: self.page_token,
        })
    }
}

fn is_canonical_activity_type(value: &str) -> bool {
    const ACTIVITY_TYPES: &[&str] = &[
        "FILL", "TRANS", "MISC", "ACATC", "ACATS", "CFEE", "CGD", "CSD", "CSW", "DIV", "DIVCGL",
        "DIVCGS", "DIVFEE", "DIVFT", "DIVNRA", "DIVROC", "DIVTW", "DIVTXEX", "FEE", "INT",
        "INTNRA", "INTTW", "JNL", "JNLC", "JNLS", "MA", "NC", "OPASN", "OPCA", "OPCSH", "OPEXC",
        "OPEXP", "OPTRD", "PTC", "PTR", "REORG", "SPIN", "SPLIT", "FOPT",
    ];

    ACTIVITY_TYPES.contains(&value)
}
