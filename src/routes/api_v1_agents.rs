//! The Agents API (`cargo offrails generate api agents`): its router, `paths::` helpers and `ROUTES` rows.
//! src/routes.rs wires it in with one `// api_v1_agents` line at `route mods`, `routes`, `paths`, `route tables`.

use axum::{Router, routing::get};

use super::{Access, Route};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/agents",
            get(crate::controllers::api_v1_agents::index)
                .post(crate::controllers::api_v1_agents::create),
        )
        .route(
            "/api/v1/agents/{id}",
            get(crate::controllers::api_v1_agents::show)
                .patch(crate::controllers::api_v1_agents::update)
                .delete(crate::controllers::api_v1_agents::destroy),
        )
        .layer(axum::middleware::map_response(
            crate::controllers::api::no_store,
        ))
}

/// URL helpers; `crate::routes::paths` re-exports them.
pub mod paths {
    pub fn api_agents() -> String {
        "/api/v1/agents".into()
    }

    pub fn api_agent(id: i64) -> String {
        format!("/api/v1/agents/{id}")
    }
}

#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET",  "/api/v1/agents",      "api_v1_agents::index",   Access::User),
    ("POST", "/api/v1/agents",      "api_v1_agents::create",  Access::User),
    ("GET",  "/api/v1/agents/{id}", "api_v1_agents::show",    Access::User),
    ("PATCH", "/api/v1/agents/{id}", "api_v1_agents::update",  Access::User),
    ("DELETE", "/api/v1/agents/{id}", "api_v1_agents::destroy", Access::User),
];
