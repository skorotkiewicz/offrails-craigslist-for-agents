//! Requests (`cargo offrails generate resource requests`): its router, `paths::` helpers and `ROUTES` rows.
//! src/routes.rs wires it in with one `// requests` line at `route mods`, `routes`, `paths`, `route tables`.

use axum::{
    Router,
    routing::{get, post},
};

use super::{Access, Route};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/requests",
            get(crate::controllers::requests::index).post(crate::controllers::requests::create),
        )
        .route("/requests/new", get(crate::controllers::requests::new))
        .route(
            "/requests/{id}",
            get(crate::controllers::requests::show).post(crate::controllers::requests::update),
        )
        .route(
            "/requests/{id}/edit",
            get(crate::controllers::requests::edit),
        )
        .route(
            "/requests/{id}/delete",
            post(crate::controllers::requests::destroy),
        )
}

/// URL helpers; `crate::routes::paths` re-exports them.
pub mod paths {
    pub fn requests() -> String {
        "/requests".into()
    }

    pub fn new_request() -> String {
        "/requests/new".into()
    }

    pub fn request(id: i64) -> String {
        format!("/requests/{id}")
    }

    pub fn edit_request(id: i64) -> String {
        format!("/requests/{id}/edit")
    }

    pub fn delete_request(id: i64) -> String {
        format!("/requests/{id}/delete")
    }
}

#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET",  "/requests",             "requests::index",   Access::User),
    ("GET",  "/requests/new",         "requests::new",     Access::User),
    ("POST", "/requests",             "requests::create",  Access::User),
    ("GET",  "/requests/{id}",        "requests::show",    Access::User),
    ("GET",  "/requests/{id}/edit",   "requests::edit",    Access::User),
    ("POST", "/requests/{id}",        "requests::update",  Access::User),
    ("POST", "/requests/{id}/delete", "requests::destroy", Access::User),
];
