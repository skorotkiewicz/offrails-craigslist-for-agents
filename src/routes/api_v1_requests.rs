//! The Requests API (`cargo offrails generate api requests`): its router, `paths::` helpers and `ROUTES` rows.
//! src/routes.rs wires it in with one `// api_v1_requests` line at `route mods`, `routes`, `paths`, `route tables`.

use axum::{Router, routing::get};

use super::{Access, Route};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/requests",
            get(crate::controllers::api_v1_requests::index)
                .post(crate::controllers::api_v1_requests::create),
        )
        .route(
            "/api/v1/requests/{id}",
            get(crate::controllers::api_v1_requests::show)
                .patch(crate::controllers::api_v1_requests::update)
                .delete(crate::controllers::api_v1_requests::destroy),
        )
        .layer(axum::middleware::map_response(
            crate::controllers::api::no_store,
        ))
}

/// URL helpers; `crate::routes::paths` re-exports them.
pub mod paths {
    pub fn api_requests() -> String {
        "/api/v1/requests".into()
    }

    pub fn api_request(id: i64) -> String {
        format!("/api/v1/requests/{id}")
    }
}

#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET",  "/api/v1/requests",      "api_v1_requests::index",   Access::User),
    ("POST", "/api/v1/requests",      "api_v1_requests::create",  Access::User),
    ("GET",  "/api/v1/requests/{id}", "api_v1_requests::show",    Access::User),
    ("PATCH", "/api/v1/requests/{id}", "api_v1_requests::update",  Access::User),
    ("DELETE", "/api/v1/requests/{id}", "api_v1_requests::destroy", Access::User),
];
