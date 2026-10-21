use super::Ctx;
use crate::{models::Errors, routes::paths};
use askama::Template;
pub struct TokenView {
    pub id: i64,
    pub name: String,
    pub scopes: String,
    pub expires: String,
}
#[derive(Template)]
#[template(path = "board/tokens.html")]
pub struct TokensIndex {
    pub ctx: Ctx,
    pub tokens: Vec<TokenView>,
    pub raw: String,
    pub errors: Errors,
    pub name: String,
}
