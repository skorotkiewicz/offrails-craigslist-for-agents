use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    kernel::flash::Flash,
    models::{Page, application, request},
    routes::paths,
    state::AppState,
    views::{self, Ctx, board::*},
};
use axum::{
    Form,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};

#[tracing::instrument(skip_all)]
pub async fn create_application(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
    Form(mut params): Form<application::ApplicationParams>,
) -> Result<Response, AppError> {
    s.rate_limit
        .check(
            &format!("applications:{}", user.id),
            30,
            std::time::Duration::from_secs(60),
        )
        .await?;
    // The URL is authoritative. A hidden field cannot redirect this application to another request.
    params.request_id = id.to_string();
    let result = match params.validate() {
        Ok(new) => application::create(&s.db, &user, new).await,
        Err(e) => Err(AppError::Invalid(e)),
    };
    match result {
        Ok(_) => Ok((
            Flash::notice("Application sent. You can track it on your board."),
            Redirect::to(&paths::task(id)),
        )
            .into_response()),
        Err(AppError::Invalid(errors)) => {
            let mut response = super::index::detail(&s, ctx, id, params, errors).await?;
            *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
            response
                .headers_mut()
                .insert("hx-push-url", header::HeaderValue::from_static("false"));
            Ok(response)
        }
        Err(e) => Err(e),
    }
}

#[tracing::instrument(skip_all)]
pub async fn applications(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
    Query(page): Query<Page>,
) -> Result<Response, AppError> {
    let row = request::find_for(&s.db, &user, id).await?;
    let (rows, more) = application::for_request(&s.db, &user, id, page).await?;
    Ok(views::render(ApplicationsIndex {
        ctx: ctx.titled("Applications").noindex(),
        task: TaskView::new(&row, None),
        applications: rows.iter().map(Into::into).collect(),
        page: page.number(),
        more,
    })?
    .into_response())
}

#[tracing::instrument(skip_all)]
pub async fn select(
    State(s): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let selected = application::select_for(&s.db, &user, id).await?;
    Ok((
        Flash::notice(
            "Person selected. Use their contact details to arrange the work and payment.",
        ),
        Redirect::to(&paths::task_applications(selected.request_id)),
    )
        .into_response())
}

#[tracing::instrument(skip_all)]
pub async fn close(
    State(s): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    request::close_for(&s.db, &user, id).await?;
    Ok((
        Flash::notice("Request closed. Your applications have been kept."),
        Redirect::to(&paths::task(id)),
    )
        .into_response())
}

/// Remember the public task across login, then return to its application form.
#[tracing::instrument(skip_all)]
pub async fn start_application(
    State(s): State<AppState>,
    RequireUser(_user): RequireUser, // unscoped: AC-board-1 public task; auth remembers the return path
    Path(id): Path<i64>,
) -> Result<Redirect, AppError> {
    request::public_request(&s.db, id).await?;
    Ok(Redirect::to(&format!("{}#apply", paths::task(id))))
}
