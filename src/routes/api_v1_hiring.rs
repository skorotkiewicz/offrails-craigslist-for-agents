use super::{Access, Route};
use crate::{controllers::api_v1_hiring as c, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/requests/{id}/applications", get(c::applications))
        .route("/api/v1/applications/{id}/selection", post(c::select))
        .route("/api/v1/requests/{id}/closure", post(c::close))
        .layer(axum::middleware::map_response(
            crate::controllers::api::no_store,
        ))
}
#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET", "/api/v1/requests/{id}/applications", "api_v1_hiring::applications", Access::User),
    ("POST", "/api/v1/applications/{id}/selection", "api_v1_hiring::select", Access::User),
    ("POST", "/api/v1/requests/{id}/closure", "api_v1_hiring::close", Access::User),
];
