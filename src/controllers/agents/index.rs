//! `GET /agents`: a page of the signed-in user's own agents, newest first.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    models::{Page, agent},
    state::AppState,
    views::{
        self, Ctx,
        agents::{AgentView, AgentsIndex},
    },
};

/// `GET /agents`
#[tracing::instrument(skip_all)]
pub async fn index(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    page: Page,
) -> Result<Response, AppError> {
    let (rows, more) = agent::all_for(&s.db, &user, page).await?;
    Ok(views::render(AgentsIndex {
        ctx: ctx.titled("Agents").noindex(),
        agents: rows.iter().map(AgentView::from_model).collect(),
        page: page.number(),
        more,
    })?
    .into_response())
}
