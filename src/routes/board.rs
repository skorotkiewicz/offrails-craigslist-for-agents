use super::{Access, Route};
use crate::{controllers::board as c, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tasks/{id}", get(c::show))
        .route("/tasks/{id}/apply", get(c::start_application))
        .route(
            "/tasks/{id}/applications",
            get(c::applications).post(c::create_application),
        )
        .route("/applications/{id}/selection", post(c::select))
        .route("/tasks/{id}/closure", post(c::close))
        .route("/dashboard", get(c::dashboard))
        .route("/for-agents", get(c::guide))
}
pub mod paths {
    pub fn apply_task(id: i64) -> String {
        format!("/tasks/{id}/apply")
    }
    pub fn task(id: i64) -> String {
        format!("/tasks/{id}")
    }
    pub fn task_applications(id: i64) -> String {
        format!("/tasks/{id}/applications")
    }
    pub fn select_application(id: i64) -> String {
        format!("/applications/{id}/selection")
    }
    pub fn close_task(id: i64) -> String {
        format!("/tasks/{id}/closure")
    }
    pub fn dashboard() -> String {
        "/dashboard".into()
    }
    pub fn agent_guide() -> String {
        "/for-agents".into()
    }
}
#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET", "/tasks/{id}", "board::show", Access::Public),
    ("GET", "/tasks/{id}/apply", "board::start_application", Access::User),
    ("GET", "/tasks/{id}/applications", "board::applications", Access::User),
    ("POST", "/tasks/{id}/applications", "board::create_application", Access::User),
    ("POST", "/applications/{id}/selection", "board::select", Access::User),
    ("POST", "/tasks/{id}/closure", "board::close", Access::User),
    ("GET", "/dashboard", "board::dashboard", Access::User),
    ("GET", "/for-agents", "board::guide", Access::Public),
];
