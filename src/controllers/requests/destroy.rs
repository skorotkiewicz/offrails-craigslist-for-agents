//! `POST /requests/{id}/delete`: deleting the signed-in user's own request.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    controllers::auth::RequireUser, error::AppError, kernel::flash::Flash, models::request,
    routes::paths, state::AppState,
};

/// `POST /requests/{id}/delete`
#[tracing::instrument(skip_all)]
pub async fn destroy(
    State(s): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = request::find_for(&s.db, &user, id).await?;
    request::destroy_for(&s.db, &user, record).await?;
    Ok((
        Flash::notice("Request cancelled. Its applications have been kept."),
        Redirect::to(&paths::requests()),
    )
        .into_response())
}
