//! `GET /requests`: a page of the signed-in user's own requests, newest first.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    controllers::auth::RequireUser,
    error::AppError,
    models::{Page, request},
    state::AppState,
    views::{
        self, Ctx,
        requests::{RequestView, RequestsIndex},
    },
};

/// `GET /requests`
#[tracing::instrument(skip_all)]
pub async fn index(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
    page: Page,
) -> Result<Response, AppError> {
    let (rows, more) = request::all_for(&s.db, &user, page).await?;
    Ok(views::render(RequestsIndex {
        ctx: ctx.titled("Requests").noindex(),
        requests: rows.iter().map(RequestView::from_model).collect(),
        page: page.number(),
        more,
    })?
    .into_response())
}
