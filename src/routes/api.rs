//! The API's own routes (`cargo offrails generate api`; .offrails/guides/api.md): its router, `paths::` helpers and `ROUTES` rows.
//! src/routes.rs wires it in with one `// api` line at `route mods`, `routes`, `paths`, `route tables`.

use axum::{Router, routing::get};

use super::{Access, Route};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/openapi.json",
            get(crate::controllers::api::openapi_json),
        )
        .route("/api/v1/me", get(crate::controllers::api::me))
        .layer(axum::middleware::map_response(
            crate::controllers::api::no_store,
        ))
}

/// URL helpers; `crate::routes::paths` re-exports them.
pub mod paths {
    pub fn api_openapi() -> String {
        "/api/openapi.json".into()
    }

    pub fn api_me() -> String {
        "/api/v1/me".into()
    }
}

#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET",  "/api/openapi.json", "api::openapi_json", Access::Public),
    ("GET",  "/api/v1/me",        "api::me",           Access::User),
];
