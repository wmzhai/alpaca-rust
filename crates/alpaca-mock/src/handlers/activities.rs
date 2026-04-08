use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use serde::Deserialize;

use alpaca_trade::{activities::Activity, orders::SortDirection};

use crate::auth::{AuthenticatedAccount, MockHttpError};
use crate::state::{ListActivitiesFilter, MockServerState};

#[derive(Debug, Deserialize, Default)]
pub(crate) struct ListActivitiesQuery {
    activity_types: Option<String>,
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
        query.into_filter(None),
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
        query.into_filter(Some(activity_type)),
    )))
}

impl ListActivitiesQuery {
    fn into_filter(self, activity_type: Option<String>) -> ListActivitiesFilter {
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

        ListActivitiesFilter {
            activity_types,
            date: self.date,
            until: self.until,
            after: self.after,
            direction: self.direction,
            page_size: self.page_size,
            page_token: self.page_token,
        }
    }
}
