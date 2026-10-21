//! `GET /agents/{id}`: one of the signed-in user's agents.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};

use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    models::agent,
    state::AppState,
    views::{
        self, Ctx,
        agents::{AgentView, AgentsShow},
    },
};

/// `GET /agents/{id}`
#[tracing::instrument(skip_all)]
pub async fn show(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = agent::find_for(&s.db, &user, id).await?;
    Ok(views::render(AgentsShow {
        ctx: ctx.titled(&record.label()).noindex(),
        agent: AgentView::from_model(&record),
    })?
    .into_response())
}
