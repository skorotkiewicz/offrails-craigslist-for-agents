//! `GET /requests/new` and `POST /requests`: the form, and creating the signed-in user's own request.

use axum::{
    Form,
    extract::State,
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
    views::{self, Ctx, requests::RequestsNew},
};

/// `GET /requests/new`
#[tracing::instrument(skip_all)]
pub async fn new(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
) -> Result<Response, AppError> {
    Ok(views::render(RequestsNew {
        ctx: ctx.titled("New request").noindex(),
        params: RequestParams::default(),
        errors: Errors::default(),
        action: paths::requests(),
        choices: choices(&s, &user).await?,
    })?
    .into_response())
}

/// `POST /requests`: 303 to the new request, or the form again with 422 and the errors.
#[tracing::instrument(skip_all)]
pub async fn create(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Form(params): Form<RequestParams>,
) -> Result<Response, AppError> {
    let result = match params.validate() {
        Ok(new) => request::create(&s.db, &user, new).await,
        Err(errors) => Err(AppError::Invalid(errors)),
    };
    match result {
        Ok(record) => Ok((
            Flash::notice("Request was successfully created."),
            Redirect::to(&paths::request(record.id)),
        )
            .into_response()),
        Err(AppError::Invalid(errors)) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            views::render(RequestsNew {
                ctx: ctx.titled("New request").noindex(),
                params,
                errors,
                action: paths::requests(),
                choices: choices(&s, &user).await?,
            })?,
        )
            .into_response()),
        Err(e) => Err(e),
    }
}
