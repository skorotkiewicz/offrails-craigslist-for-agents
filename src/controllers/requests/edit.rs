//! `GET /requests/{id}/edit` and `POST /requests/{id}`: the form, and saving changes to the signed-in user's own request.

use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};

use super::choices;
use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    kernel::flash::Flash,
    models::{Errors, request, request::RequestParams},
    routes::paths,
    state::AppState,
    views::{self, Ctx, requests::RequestsEdit},
};

/// `GET /requests/{id}/edit`
#[tracing::instrument(skip_all)]
pub async fn edit(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = request::find_for(&s.db, &user, id).await?;
    Ok(views::render(RequestsEdit {
        ctx: ctx.titled(&format!("Edit {}", record.label())).noindex(),
        id: record.id,
        params: RequestParams::from(&record),
        errors: Errors::default(),
        action: paths::request(record.id),
        choices: choices(&s, &user).await?,
    })?
    .into_response())
}

/// `POST /requests/{id}`: 303 to the request, or the form again with 422 and the errors.
#[tracing::instrument(skip_all)]
pub async fn update(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
    Form(params): Form<RequestParams>,
) -> Result<Response, AppError> {
    let record = request::find_for(&s.db, &user, id).await?;
    let result = match params.validate() {
        Ok(new) => request::update_for(&s.db, &user, record, new).await,
        Err(errors) => Err(AppError::Invalid(errors)),
    };
    match result {
        Ok(record) => Ok((
            Flash::notice("Request was successfully updated."),
            Redirect::to(&paths::request(record.id)),
        )
            .into_response()),
        Err(AppError::Invalid(errors)) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            views::render(RequestsEdit {
                ctx: ctx.titled("Edit request").noindex(),
                id,
                params,
                errors,
                action: paths::request(id),
                choices: choices(&s, &user).await?,
            })?,
        )
            .into_response()),
        Err(e) => Err(e),
    }
}
