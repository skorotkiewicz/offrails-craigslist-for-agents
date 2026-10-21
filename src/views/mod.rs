//! What the client receives: Askama page structs, one file per resource, plus the layout context.

pub mod agents; // agents
pub mod agents_json; // agents
pub mod auth;
pub mod board; // board
pub mod errors;
pub mod gallery;
pub mod helpers;
pub mod home;
pub mod login_links;
pub mod requests; // requests
pub mod requests_json; // requests
pub mod tokens; // tokens
// samples: view mods

use crate::error::AppError;
use crate::kernel::assets::Assets;
use crate::kernel::flash::FlashMessage;

pub const DEFAULT_DESCRIPTION: &str = "Agents need humans. Find your next task.";

pub fn render(t: impl askama::Template) -> Result<axum::response::Html<String>, AppError> {
    Ok(axum::response::Html(t.render()?))
}

/// The layout context every page struct carries. The layout reads only from `ctx`.
#[derive(Debug, Clone)]
pub struct Ctx {
    pub title: String,
    pub description: String,
    pub path: String,
    pub app_url: String,
    pub flash: Option<FlashMessage>,
    pub assets: Assets,
    pub dev: bool,
    pub noindex: bool,
    /// Development only: static/app.css is missing, empty or stale (dev-tools.md#introspection).
    pub css_warning: Option<String>,
    /// `<html data-theme>`: the `offrails_theme` cookie (kernel/theme.rs).
    pub theme: crate::kernel::theme::Theme,
    /// The dev dashboard's browser beacon (`kernel/dashboard/rum.rs`); `None` when it's off.
    pub dashboard_rum: Option<crate::kernel::dashboard::rum::Beacon>,
    /// The signed-in person (auth-core), for the nav and pages that greet them.
    pub current_user: Option<crate::models::user::Model>,
    /// Development with the local mailer: auth screens link to the dev mailbox.
    pub dev_mailbox: bool,
    /// `static/auth.js`, digested: the app layout loads it on every page (`samples: app head`).
    pub auth_js: String,
    /// Anyone may create an account (`config.auth.signup`): sign-up links show only then.
    pub signup_open: bool,
    // samples: ctx fields
}

impl Ctx {
    pub fn titled(mut self, t: &str) -> Self {
        self.title = t.to_owned();
        self
    }

    /// Collapses whitespace and caps at 300 chars (search engines show ~160).
    pub fn described(mut self, d: &str) -> Self {
        let collapsed: String = d.split_whitespace().collect::<Vec<_>>().join(" ");
        self.description = collapsed.chars().take(300).collect();
        self
    }

    /// Pages behind a login: `<meta name="robots" content="noindex">`.
    pub fn noindex(mut self) -> Self {
        self.noindex = true;
        self
    }

    /// "Posts · My App", or "My App: <tagline>" on the home page.
    pub fn page_title(&self) -> String {
        if self.title.is_empty() {
            format!(
                "{}: {}",
                crate::config::APP_NAME,
                crate::config::APP_TAGLINE
            )
        } else {
            format!("{} · {}", self.title, crate::config::APP_NAME)
        }
    }

    pub fn canonical(&self) -> String {
        format!("{}{}", self.app_url, self.path)
    }

    pub fn absolute(&self, path: &str) -> String {
        format!("{}{path}", self.app_url)
    }
}
