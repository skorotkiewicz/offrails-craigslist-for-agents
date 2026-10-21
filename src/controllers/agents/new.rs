//! `GET /agents/new` and `POST /agents`: the form, and creating the signed-in user's own agent.

use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    kernel::flash::Flash,
    models::{Errors, agent, agent::AgentParams},
    routes::paths,
    state::AppState,
    views::{self, Ctx, agents::AgentsNew},
};

/// `GET /agents/new`
#[tracing::instrument(skip_all)]
pub async fn new(
    ctx: Ctx,
    RequireUser(_user): RequireUser, // unscoped: an empty form reads nothing private
) -> Result<Response, AppError> {
    Ok(views::render(AgentsNew {
        ctx: ctx.titled("New agent").noindex(),
        params: AgentParams::default(),
        errors: Errors::default(),
        action: paths::agents(),
    })?
    .into_response())
}

/// `POST /agents`: 303 to the new agent, or the form again with 422 and the errors.
#[tracing::instrument(skip_all)]
pub async fn create(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Form(params): Form<AgentParams>,
) -> Result<Response, AppError> {
    let result = match params.validate() {
        Ok(new) => agent::create(&s.db, &user, new).await,
        Err(errors) => Err(AppError::Invalid(errors)),
    };
    match result {
        Ok(record) => Ok((
            Flash::notice("Agent was successfully created."),
            Redirect::to(&paths::agent(record.id)),
        )
            .into_response()),
        Err(AppError::Invalid(errors)) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            views::render(AgentsNew {
                ctx: ctx.titled("New agent").noindex(),
                params,
                errors,
                action: paths::agents(),
            })?,
        )
            .into_response()),
        Err(e) => Err(e),
    }
}
