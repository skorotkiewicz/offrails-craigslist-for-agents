use crate::{
    controllers::auth::{RequireRecentAuth, RequireUser},
    error::AppError,
    kernel::flash::Flash,
    models::{Errors, User, api_token},
    routes::paths,
    state::AppState,
    views::{self, Ctx, tokens::*},
};
use axum::{
    Form,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
#[derive(Default, Deserialize)]
pub struct TokenParams {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub scope: String,
}

async fn page(
    s: &AppState,
    ctx: Ctx,
    user: &User,
    raw: String,
    errors: Errors,
    name: String,
) -> Result<Response, AppError> {
    let tokens = api_token::all_for(&s.db, user)
        .await?
        .iter()
        .map(|t| TokenView {
            id: t.id,
            name: t.name.clone(),
            scopes: t.scope_list().join(", "),
            expires: t.expires_at.map_or_else(
                || "No expiry".into(),
                |at| at.format("%b %d, %Y").to_string(),
            ),
        })
        .collect();
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        views::render(TokensIndex {
            ctx: ctx.titled("API tokens").noindex(),
            tokens,
            raw,
            errors,
            name,
        })?,
    )
        .into_response())
}
#[tracing::instrument(skip_all)]
pub async fn index(
    State(s): State<AppState>,
    ctx: Ctx,
    RequireUser(user): RequireUser,
) -> Result<Response, AppError> {
    page(
        &s,
        ctx,
        &user,
        String::new(),
        Errors::default(),
        String::new(),
    )
    .await
}
#[tracing::instrument(skip_all)]
pub async fn create(
    State(s): State<AppState>,
    ctx: Ctx,
    recent: RequireRecentAuth,
    Form(params): Form<TokenParams>,
) -> Result<Response, AppError> {
    s.rate_limit
        .check(
            &format!("issue_token:{}", recent.user.id),
            10,
            std::time::Duration::from_secs(180),
        )
        .await?;
    let mut errors = Errors::default();
    if params.name.trim().is_empty() || params.name.chars().count() > 100 {
        errors.add("name", "must be between 1 and 100 characters");
    }
    if !["read", "write"].contains(&params.scope.as_str()) {
        errors.add("scope", "must be read or write");
    }
    if api_token::all_for(&s.db, &recent.user).await?.len() >= 100 {
        errors.add("tokens", "must be revoked before creating more than 100");
    }
    if !errors.is_empty() {
        let mut res = page(&s, ctx, &recent.user, String::new(), errors, params.name).await?;
        *res.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
        res.headers_mut()
            .insert("hx-push-url", axum::http::HeaderValue::from_static("false"));
        return Ok(res);
    }
    let (_, raw) = api_token::issue(
        &s.db,
        &recent.user,
        &params.name,
        params.scope == "write",
        Some(Utc::now() + Duration::days(90)),
        Utc::now(),
    )
    .await?;
    let mut response = page(&s, ctx, &recent.user, raw, Errors::default(), String::new()).await?;
    response
        .headers_mut()
        .insert("hx-push-url", axum::http::HeaderValue::from_static("false"));
    Ok(response)
}
#[tracing::instrument(skip_all)]
pub async fn destroy(
    State(s): State<AppState>,
    recent: RequireRecentAuth,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    api_token::revoke_for(&s.db, &recent.user, id).await?;
    Ok((
        Flash::notice("API token revoked."),
        Redirect::to(&paths::tokens()),
    )
        .into_response())
}
