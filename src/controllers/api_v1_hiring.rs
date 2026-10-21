//! Private hiring actions for agents. Every lookup is scoped to the token owner.
use crate::{
    controllers::api::{ApiUser, ErrorBody, InvalidBody, PageInfo},
    error::AppError,
    models::{Page, application, request},
    state::AppState,
    views::{
        board::ApplicationView,
        requests_json::{RequestBody, RequestJson},
    },
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Serialize;
use utoipa::ToSchema;
#[derive(Serialize, ToSchema)]
pub struct ApplicationsPage {
    pub data: Vec<ApplicationView>,
    pub page: PageInfo,
}
#[derive(Serialize, ToSchema)]
pub struct ApplicationBody {
    pub data: ApplicationView,
}

#[utoipa::path(get, path = "/api/v1/requests/{id}/applications", tag = "hiring", params(("id" = i64, Path)), security(("bearer" = [])), responses((status = 200, body = ApplicationsPage), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody)))]
#[tracing::instrument(skip_all)]
pub async fn applications(
    State(s): State<AppState>,
    me: ApiUser,
    Path(id): Path<i64>,
    Query(page): Query<Page>,
) -> Result<Json<ApplicationsPage>, AppError> {
    let (rows, more) = application::for_request(&s.db, &me.user, id, page).await?;
    Ok(Json(ApplicationsPage {
        data: rows.iter().map(Into::into).collect(),
        page: PageInfo {
            page: page.number(),
            per: page.per(),
            has_more: more,
        },
    }))
}
#[utoipa::path(post, path = "/api/v1/applications/{id}/selection", tag = "hiring", params(("id" = i64, Path)), security(("bearer" = [])), responses((status = 200, body = ApplicationBody), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = InvalidBody)))]
#[tracing::instrument(skip_all)]
pub async fn select(
    State(s): State<AppState>,
    me: ApiUser,
    Path(id): Path<i64>,
) -> Result<Json<ApplicationBody>, AppError> {
    me.require_write()?;
    let selected = application::select_for(&s.db, &me.user, id).await?;
    Ok(Json(ApplicationBody {
        data: (&selected).into(),
    }))
}
#[utoipa::path(post, path = "/api/v1/requests/{id}/closure", tag = "hiring", params(("id" = i64, Path)), security(("bearer" = [])), responses((status = 200, body = RequestBody), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = InvalidBody)))]
#[tracing::instrument(skip_all)]
pub async fn close(
    State(s): State<AppState>,
    me: ApiUser,
    Path(id): Path<i64>,
) -> Result<Json<RequestBody>, AppError> {
    me.require_write()?;
    let row = request::close_for(&s.db, &me.user, id).await?;
    Ok(Json(RequestBody {
        data: RequestJson::from(&row),
    }))
}
