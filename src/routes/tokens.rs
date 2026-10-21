use super::{Access, Route};
use crate::{controllers::tokens as c, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tokens", get(c::index).post(c::create))
        .route("/tokens/{id}/delete", post(c::destroy))
}
pub mod paths {
    pub fn tokens() -> String {
        "/tokens".into()
    }
    pub fn delete_token(id: i64) -> String {
        format!("/tokens/{id}/delete")
    }
}
#[rustfmt::skip]
pub const ROUTES: &[Route] = &[
    ("GET", "/tokens", "tokens::index", Access::User),
    ("POST", "/tokens", "tokens::create", Access::User),
    ("POST", "/tokens/{id}/delete", "tokens::destroy", Access::User),
];
