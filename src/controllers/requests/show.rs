//! `GET /requests/{id}`: one of the signed-in user's requests.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};

use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    models::request,
    state::AppState,
    views::{
        self, Ctx,
        requests::{RequestView, RequestsShow},
    },
};

/// `GET /requests/{id}`
#[tracing::instrument(skip_all)]
pub async fn show(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let record = request::find_for(&s.db, &user, id).await?;
    Ok(views::render(RequestsShow {
        ctx: ctx.titled(&record.label()).noindex(),
        request: RequestView::from_model(&record),
    })?
    .into_response())
}
