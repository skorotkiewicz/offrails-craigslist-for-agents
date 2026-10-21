use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    models::{Errors, Page, application, request},
    state::AppState,
    views::{self, Ctx, board::*},
};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};

#[tracing::instrument(skip_all)]
pub async fn index(
    State(s): State<AppState>,
    ctx: Ctx,
    Query(filter): Query<request::BoardFilter>,
    Query(page): Query<Page>,
) -> Result<Response, AppError> {
    let (rows, more) = request::browse(&s.db, &filter, page).await?;
    let tasks = rows
        .iter()
        .map(|(r, a)| TaskView::new(r, a.as_ref()))
        .collect();
    Ok(views::render(BoardIndex {
        ctx,
        tasks,
        filter,
        page: page.number(),
        more,
    })?
    .into_response())
}

#[tracing::instrument(skip_all)]
pub async fn show(
    State(s): State<AppState>,
    ctx: Ctx,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    detail(
        &s,
        ctx,
        id,
        application::ApplicationParams::default(),
        Errors::default(),
    )
    .await
}

pub(super) async fn detail(
    s: &AppState,
    ctx: Ctx,
    id: i64,
    params: application::ApplicationParams,
    errors: Errors,
) -> Result<Response, AppError> {
    let (row, agent) = request::public_request(&s.db, id).await?;
    let owner = ctx
        .current_user
        .as_ref()
        .is_some_and(|u| u.id == row.user_id);
    let applied = match ctx.current_user.as_ref() {
        Some(user) => application::existing_for(&s.db, user, id).await?.is_some(),
        None => false,
    };
    Ok(views::render(TaskDetail {
        ctx: ctx.titled(&row.title),
        task: TaskView::new(&row, agent.as_ref()),
        owner,
        applied,
        params,
        errors,
    })?
    .into_response())
}

#[tracing::instrument(skip_all)]
pub async fn dashboard(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Query(page): Query<Page>,
) -> Result<Response, AppError> {
    let (owned, owned_more) = request::all_for(&s.db, &user, page).await?;
    let (submitted, app_more) = application::submitted_for(&s.db, &user, page).await?;
    let tasks = owned.iter().map(|r| TaskView::new(r, None)).collect();
    let applications = submitted
        .iter()
        .filter_map(|(a, r)| {
            r.as_ref().map(|task| SubmittedView {
                application: a.into(),
                task: TaskView::new(task, None),
            })
        })
        .collect();
    Ok(views::render(Dashboard {
        ctx: ctx.titled("Your board").noindex(),
        tasks,
        applications,
        page: page.number(),
        more: owned_more || app_more,
    })?
    .into_response())
}

#[tracing::instrument(skip_all)]
pub async fn guide(ctx: Ctx) -> Result<Response, AppError> {
    Ok(views::render(AgentGuide {
        ctx: ctx.titled("For agents"),
    })?
    .into_response())
}
