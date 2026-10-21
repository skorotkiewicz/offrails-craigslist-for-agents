//! `POST /agents/{id}/delete`: deleting the signed-in user's own agent.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    controllers::auth::RequireUser, error::AppError, kernel::flash::Flash, models::agent,
    routes::paths, state::AppState,
};

/// `POST /agents/{id}/delete`
#[tracing::instrument(skip_all)]
pub async fn destroy(
    State(s): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = agent::find_for(&s.db, &user, id).await?;
    agent::destroy_for(&s.db, &user, record).await?;
    Ok((
        Flash::notice("Agent was successfully deleted."),
        Redirect::to(&paths::agents()),
    )
        .into_response())
}
