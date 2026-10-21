//! `GET /agents/{id}/edit` and `POST /agents/{id}`: the form, and saving changes to the signed-in user's own agent.

use axum::{
    Form,
    extract::{Path, State},
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
    views::{self, Ctx, agents::AgentsEdit},
};

/// `GET /agents/{id}/edit`
#[tracing::instrument(skip_all)]
pub async fn edit(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = agent::find_for(&s.db, &user, id).await?;
    Ok(views::render(AgentsEdit {
        ctx: ctx.titled(&format!("Edit {}", record.label())).noindex(),
        id: record.id,
        params: AgentParams::from(&record),
        errors: Errors::default(),
        action: paths::agent(record.id),
    })?
    .into_response())
}

/// `POST /agents/{id}`: 303 to the agent, or the form again with 422 and the errors.
#[tracing::instrument(skip_all)]
pub async fn update(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
    Form(params): Form<AgentParams>,
) -> Result<Response, AppError> {
    let record = agent::find_for(&s.db, &user, id).await?;
    let result = match params.validate() {
        Ok(new) => agent::update_for(&s.db, &user, record, new).await,
        Err(errors) => Err(AppError::Invalid(errors)),
    };
    match result {
        Ok(record) => Ok((
            Flash::notice("Agent was successfully updated."),
            Redirect::to(&paths::agent(record.id)),
        )
            .into_response()),
        Err(AppError::Invalid(errors)) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            views::render(AgentsEdit {
                ctx: ctx.titled("Edit agent").noindex(),
                id,
                params,
                errors,
                action: paths::agent(id),
            })?,
        )
            .into_response()),
        Err(e) => Err(e),
    }
}
