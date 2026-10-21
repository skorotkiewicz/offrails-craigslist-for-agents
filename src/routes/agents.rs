//! Agents (`cargo offrails generate resource agents`): its router, `paths::` helpers and `ROUTES` rows.
//! src/routes.rs wires it in with one `// agents` line at `route mods`, `routes`, `paths`, `route tables`.

use axum::{
    Router,
    routing::{get, post},
};

use super::{Access, Route};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/agents",
            get(crate::controllers::agents::index).post(crate::controllers::agents::create),
        )
        .route("/agents/new", get(crate::controllers::agents::new))
        .route(
            "/agents/{id}",
            get(crate::controllers::agents::show).post(crate::controllers::agents::update),
        )
        .route("/agents/{id}/edit", get(crate::controllers::agents::edit))
        .route(
            "/agents/{id}/delete",
            post(crate::controllers::agents::destroy),
        )
}

/// URL helpers; `crate::routes::paths` re-exports them.
pub mod paths {
    pub fn agents() -> String {
        "/agents".into()
    }

    pub fn new_agent() -> String {
        "/agents/new".into()
    }

    pub fn agent(id: i64) -> String {
        format!("/agents/{id}")
    }

    pub fn edit_agent(id: i64) -> String {
        format!("/agents/{id}/edit")
    }

    pub fn delete_agent(id: i64) -> String {
        format!("/agents/{id}/delete")
    }
}

#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET",  "/agents",             "agents::index",   Access::User),
    ("GET",  "/agents/new",         "agents::new",     Access::User),
    ("POST", "/agents",             "agents::create",  Access::User),
    ("GET",  "/agents/{id}",        "agents::show",    Access::User),
    ("GET",  "/agents/{id}/edit",   "agents::edit",    Access::User),
    ("POST", "/agents/{id}",        "agents::update",  Access::User),
    ("POST", "/agents/{id}/delete", "agents::destroy", Access::User),
];
