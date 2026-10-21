//! THE router: every URL the app serves, read top to bottom. (`config/routes.rb`)
//!
//! `htmx` mode: HTML forms only send GET and POST, so update is `POST /<things>/{id}` and
//! destroy is `POST /<things>/{id}/delete`.

use std::time::Duration;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, StatusCode},
    middleware::{from_fn, from_fn_with_state, map_response},
    response::IntoResponse,
    routing::{get, post},
};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    csrf::CsrfLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::{
    controllers::{dev, errors, health, home, meta, migrations, theme},
    error::{self, AppError},
    kernel::{assets, flash, hosts, redact, security},
    state::AppState,
};

// Feature route files, one per feature (src/routes/<feature>.rs: its router, `paths::` helpers
// and `ROUTES` rows), each wired in by one line per spot, marked with the feature's name.
pub mod agents; // agents
pub mod api; // api
pub mod api_v1_agents; // agents
pub mod api_v1_hiring;
pub mod api_v1_requests; // requests
pub mod board; // board
pub mod requests; // requests
pub mod tokens; // tokens // api_v1_hiring
// samples: route mods

/// Who may call a route. The security suite requests every route as each actor
/// (`.offrails/guides/test-standards.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Public,
    User,
    Admin,
    Dev,
}

impl Access {
    pub fn as_str(self) -> &'static str {
        match self {
            Access::Public => "public",
            Access::User => "user",
            Access::Admin => "admin",
            Access::Dev => "dev",
        }
    }
}

pub fn router(state: AppState) -> anyhow::Result<Router> {
    let config = state.config.clone();
    #[allow(unused_mut)] // samples add routes below
    let mut app = Router::new()
        .route("/", get(home::index))
        .route("/up", get(health::up))
        .route("/robots.txt", get(meta::robots))
        .route("/sitemap.xml", get(meta::sitemap))
        // The theme switch's no-JS fallback; kernel pages (/admin/jobs too) and the app layout.
        .route("/theme", post(theme::update))
        // Browsers ask for it at the root whatever the <link> says.
        .route_service("/favicon.ico", ServeFile::new("static/favicon.ico"));
    // /admin/dashboard: the read-only dashboard. Open in development; in production only with
    // DASHBOARD_IN_PRODUCTION=true and an admin gate; otherwise not mounted (404).
    if crate::kernel::dashboard::access::admin_mounted(&config) {
        app = app.merge(crate::kernel::dashboard::web::admin_router().route_layer(
            axum::middleware::from_extractor_with_state::<
                crate::kernel::dashboard::access::DashboardAccess,
                _,
            >(state.clone()),
        ));
        // The browser beacon's endpoint (DASHBOARD_RUM=true in production; development posts to
        // /dev/dashboard/rum). It answers 404 unless the beacon is on.
        app = app.route(
            "/rum",
            axum::routing::post(crate::kernel::dashboard::rum::collect),
        );
    }
    // Accounts (auth-core). Sign-in forms come from the method samples; see their routes below.
    app = app
        .route("/session/new", get(crate::controllers::sessions::new))
        .route(
            "/session/delete",
            axum::routing::post(crate::controllers::sessions::destroy),
        )
        .route("/account", get(crate::controllers::account::show))
        .route(
            "/account/confirm",
            get(crate::controllers::account::confirm),
        )
        .route(
            "/account/sessions/delete-others",
            axum::routing::post(crate::controllers::account::revoke_others),
        )
        .route(
            "/account/sessions/{id}/delete",
            axum::routing::post(crate::controllers::account::revoke),
        )
        .route(
            "/account/email/edit",
            get(crate::controllers::account::edit_email),
        )
        .route(
            "/account/email",
            axum::routing::post(crate::controllers::account::update_email),
        )
        .route(
            "/account/email/confirm/{token}",
            get(crate::controllers::account::show_email_change)
                .post(crate::controllers::account::confirm_email_change),
        )
        .route(
            "/security/not-me/{token}",
            get(crate::controllers::security::show).post(crate::controllers::security::create),
        );
    // Magic links (auth-magic-link).
    app = app
        .route(
            "/login_links",
            axum::routing::post(crate::controllers::login_links::create),
        )
        .route(
            "/login_links/sent",
            get(crate::controllers::login_links::sent),
        )
        .route(
            "/login_links/code",
            axum::routing::post(crate::controllers::login_links::code),
        )
        .route(
            "/login_links/{token}",
            get(crate::controllers::login_links::show)
                .post(crate::controllers::login_links::consume),
        )
        .route(
            "/account/confirm/email",
            axum::routing::post(crate::controllers::login_links::send_reauth_code),
        )
        .route(
            "/account/confirm/email/code",
            axum::routing::post(crate::controllers::login_links::confirm_reauth_code),
        );
    // /admin/jobs: open in development; elsewhere behind AdminAccess, or not mounted at all.
    if crate::controllers::admin_access::dashboard_enabled(&config) {
        app = app.merge(
            crate::kernel::jobs_web::router(crate::jobs::recurring::SCHEDULE).route_layer(
                axum::middleware::from_extractor_with_state::<
                    crate::controllers::admin_access::AdminAccess,
                    _,
                >(state.clone()),
            ),
        );
    } else {
        tracing::warn!(
            "jobs dashboard disabled: set APP_ADMIN_PASSWORD (or wire an admin account)"
        );
    }
    app = app.merge(agents::router()); // agents
    app = app.merge(requests::router()); // requests
    app = app.merge(api::router()); // api
    app = app.merge(api_v1_requests::router()); // requests
    app = app.merge(api_v1_agents::router()); // agents
    app = app.merge(board::router()); // board
    app = app.merge(tokens::router()); // tokens
    app = app.merge(api_v1_hiring::router()); // api_v1_hiring
    // samples: routes

    // Development only. In production these routes don't exist (a test asserts the 404s).
    if config.is_development() {
        app = app.merge(dev::router());
        // SECURITY: development only; in production these routes don't exist (test: the_dashboard_is_404_in_production_by_default).
        app = app.merge(crate::kernel::dashboard::web::dev_router());
        // SECURITY: development only; in production these routes don't exist (test: mailbox_routes_do_not_exist_in_production_or_test).
        app = app.merge(crate::kernel::mailbox_web::router(
            crate::mailers::previews::PREVIEWS,
        ));
        // samples: dev routers
    }

    // Digested URLs are immutable, everything else revalidates.
    let static_files = Router::new()
        .fallback_service(ServeDir::new("static"))
        .layer(from_fn_with_state(
            state.assets.clone(),
            assets::cache_headers,
        ));

    let mut csrf = CsrfLayer::new().add_trusted_origin(&config.app_url)?;
    if let Some(port) = config.dev_http_port {
        // The same app on plain http beside the TLS port (development only).
        csrf = csrf.add_trusted_origin(format!("http://localhost:{port}"))?;
    }
    let csrf = csrf.with_rejection_response(|_| AppError::Forbidden.into_response());

    let x_request_id = HeaderName::from_static("x-request-id");
    let app = app
        .nest_service("/static", static_files)
        .fallback(errors::not_found);
    // The pending-migrations guard shows a page instead of a "no such table" error. A layer
    // only wraps what exists when it is applied, so it goes after the fallback: unknown paths
    // get the page too.
    let app = if config.is_development() {
        app.layer(from_fn_with_state(state.clone(), migrations::guard))
    } else {
        app
    };
    // `.layer` calls run bottom-up: the last one added is outermost.
    Ok(app
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(from_fn(flash::middleware))
        .layer(csrf)
        // A panic becomes a shaped 500, so it sits inside `shape_errors`.
        .layer(CatchPanicLayer::custom(error::panic_response))
        .layer(from_fn_with_state(state.clone(), error::shape_errors))
        .layer(CompressionLayer::new())
        // Overload is 503 + Retry-After, never 408 (errors-and-logging.md#response-mapping).
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            Duration::from_secs(30),
        ))
        .layer(map_response(error::retry_after))
        // Development only: a Host that isn't this machine is refused (DNS rebinding).
        .layer(from_fn_with_state(
            hosts::AllowedHosts::new(&config),
            hosts::middleware,
        ))
        // Outside the panic and timeout layers, so a 500 or 503 carries the headers too.
        .layer(from_fn_with_state(
            security::Hardening::new(&config),
            security::middleware,
        ))
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        // SECURITY: the request span logs the URI with tokens redacted (emailed links carry one in
        // the path), never the raw URI (test: tokens_in_request_uris_never_reach_the_logs).
        .layer(TraceLayer::new_for_http().make_span_with(
            |req: &axum::http::Request<axum::body::Body>| {
                tracing::debug_span!(
                    "request",
                    method = %req.method(),
                    uri = %redact::uri(req.uri()),
                    version = ?req.version(),
                )
            },
        ))
        .layer(from_fn(error::log_server_errors))
        .layer(from_fn(crate::kernel::dashboard::http::middleware))
        // samples: outer layers
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        .with_state(state))
}

/// URL helpers (`*_path`). Templates, redirects and mailers use these, never string literals.
pub mod paths {
    pub fn root() -> String {
        "/".into()
    }
    pub fn up() -> String {
        "/up".into()
    }
    /// Where `AppError::Unauthorized` sends a visitor; an authentication sample routes it.
    pub fn new_session() -> String {
        "/session/new".into()
    }
    pub fn dev() -> String {
        "/dev".into()
    }
    /// The theme switch (`POST`, no-JS fallback).
    pub fn theme() -> String {
        "/theme".into()
    }
    /// The component gallery (development only).
    pub fn dev_components() -> String {
        "/dev/components".into()
    }
    pub fn dev_components_modal() -> String {
        "/dev/components/modal".into()
    }
    /// A layout demo: `sidebar`, `marketing`, `auth` or `docs` (development only).
    pub fn dev_layout(name: &str) -> String {
        format!("/dev/layouts/{name}")
    }
    /// The dev dashboard. Links into it carry `hx-boost="false"` (a kernel page has its own <head>).
    pub fn dev_dashboard() -> String {
        "/dev/dashboard".into()
    }
    /// The dev mailbox. Links into it carry `hx-boost="false"` (a kernel page has its own <head>).
    pub fn dev_mailbox() -> String {
        "/dev/mailbox".into()
    }
    /// After signing in, unless the person was headed somewhere (auth-core).
    // APP: your app's home for signed-in people (a dashboard, `/books`, …).
    pub fn after_sign_in() -> String {
        dashboard()
    }
    pub fn session_delete() -> String {
        "/session/delete".into()
    }
    pub fn account() -> String {
        "/account".into()
    }
    pub fn account_confirm() -> String {
        "/account/confirm".into()
    }
    pub fn account_session_delete(id: &i64) -> String {
        format!("/account/sessions/{id}/delete")
    }
    pub fn account_sessions_delete_others() -> String {
        "/account/sessions/delete-others".into()
    }
    pub fn account_email_edit() -> String {
        "/account/email/edit".into()
    }
    pub fn account_email() -> String {
        "/account/email".into()
    }
    pub fn email_change(token: &str) -> String {
        format!("/account/email/confirm/{token}")
    }
    pub fn not_me(token: &str) -> String {
        format!("/security/not-me/{token}")
    }
    pub fn login_links() -> String {
        "/login_links".into()
    }
    pub fn login_links_sent() -> String {
        "/login_links/sent".into()
    }
    pub fn login_links_code() -> String {
        "/login_links/code".into()
    }
    pub fn login_link(token: &str) -> String {
        format!("/login_links/{token}")
    }
    pub fn account_confirm_email() -> String {
        "/account/confirm/email".into()
    }
    pub fn account_confirm_email_code() -> String {
        "/account/confirm/email/code".into()
    }
    pub fn admin_jobs() -> String {
        "/admin/jobs".into()
    }
    pub use super::agents::paths::*; // agents
    pub use super::api::paths::*; // api
    pub use super::api_v1_agents::paths::*; // agents
    pub use super::api_v1_requests::paths::*; // requests
    pub use super::board::paths::*; // board
    pub use super::requests::paths::*; // requests
    pub use super::tokens::paths::*; // tokens
    // samples: paths
}

/// One row of the route table: method, path pattern, handler, who may call it.
pub type Route = (&'static str, &'static str, &'static str, Access);

/// The route table: the app's own rows, then each feature's (`src/routes/<feature>.rs`), in
/// order. `app routes` prints it and writes docs/routes.md. Tests assert every entry is routable
/// (not the router fallback) and enforce its access level.
pub const ROUTES: &[Route] = &concat::<{ count(TABLES) }>(TABLES);

/// The tables `ROUTES` is made of: one line per feature.
const TABLES: &[&[Route]] = &[
    APP_ROUTES,
    // agents
    agents::ROUTES,
    // requests
    requests::ROUTES,
    // api
    api::ROUTES,
    // requests
    api_v1_requests::ROUTES,
    // agents
    api_v1_agents::ROUTES,
    // board
    board::ROUTES,
    // tokens
    tokens::ROUTES,
    // api_v1_hiring
    api_v1_hiring::ROUTES,
    // samples: route tables
];

/// The kernel's rows, and the parts'.
#[rustfmt::skip]
const APP_ROUTES: &[Route] = &[
    ("GET",  "/",                    "home::index",     Access::Public),
    ("GET",  "/up",                  "health::up",      Access::Public),
    ("GET",  "/robots.txt",          "meta::robots",    Access::Public),
    ("GET",  "/sitemap.xml",         "meta::sitemap",   Access::Public),
    ("GET",  "/favicon.ico",         "static favicon",  Access::Public),
    ("POST", "/theme",               "theme::update",   Access::Public),
    ("GET",  "/dev",                 "dev::index",      Access::Dev),
    ("GET",  "/dev/routes",          "dev::routes",     Access::Dev),
    ("GET",  "/dev/system",          "dev::system",     Access::Dev),
    ("GET",  "/dev/introspect.json", "dev::introspect", Access::Dev),
    ("GET",  "/dev/reload",          "dev::reload",     Access::Dev),
    ("POST", "/dev/migrations",      "migrations::run", Access::Dev),
    ("GET",  "/dev/components",      "gallery::index",  Access::Dev),
    ("POST", "/dev/components",      "gallery::submit", Access::Dev),
    ("GET",  "/dev/components/modal", "gallery::modal", Access::Dev),
    ("GET",  "/dev/layouts/sidebar", "gallery::sidebar", Access::Dev),
    ("GET",  "/dev/layouts/marketing", "gallery::marketing", Access::Dev),
    ("GET",  "/dev/layouts/auth",    "gallery::auth",   Access::Dev),
    ("POST", "/dev/layouts/auth",    "gallery::auth_submit", Access::Dev),
    ("GET",  "/dev/layouts/docs",    "gallery::docs",   Access::Dev),
    ("GET",  "/dev/dashboard",                        "web::overview",     Access::Dev),
    ("GET",  "/dev/dashboard.json",                   "web::json",         Access::Dev),
    ("GET",  "/dev/dashboard/insights",               "web::insights_page", Access::Dev),
    ("POST", "/dev/dashboard/insights/dismiss",       "web::dismiss",      Access::Dev),
    ("GET",  "/dev/dashboard/charts",                 "web::charts_page",  Access::Dev),
    ("GET",  "/dev/dashboard/requests",               "web::requests_page", Access::Dev),
    ("GET",  "/dev/dashboard/routes",                 "web::routes_page",  Access::Dev),
    ("GET",  "/dev/dashboard/traces",                 "traces_web::explorer", Access::Dev),
    ("GET",  "/dev/dashboard/traces/compare",         "traces_web::compare", Access::Dev),
    ("GET",  "/dev/dashboard/traces/lookup",          "traces_web::lookup", Access::Dev),
    ("GET",  "/dev/dashboard/traces/{id}",            "traces_web::trace_page", Access::Dev),
    ("GET",  "/dev/dashboard/traces/{id}/waterfall",  "traces_web::waterfall_fragment", Access::Dev),
    ("GET",  "/dev/dashboard/traces/{id}/json",       "traces_web::trace_json", Access::Dev),
    ("GET",  "/dev/dashboard/methods",                "traces_web::methods_page", Access::Dev),
    ("GET",  "/dev/dashboard/upstreams",              "upstreams::upstreams_page", Access::Dev),
    ("GET",  "/dev/dashboard/security",               "threats::security_page", Access::Dev),
    ("GET",  "/dev/dashboard/frontend",               "rum::frontend_page", Access::Dev),
    ("POST", "/dev/dashboard/rum",                    "rum::collect",      Access::Dev),
    ("POST", "/rum",                                  "rum::collect",      Access::Public),
    ("GET",  "/dev/dashboard/profile",                "traces_web::profile_page", Access::Dev),
    ("POST", "/dev/dashboard/profile/toggle",         "traces_web::profile_toggle", Access::Dev),
    ("GET",  "/dev/dashboard/sql",                    "web::sql_page",     Access::Dev),
    ("GET",  "/dev/dashboard/sql/{id}",               "web::sql_show",     Access::Dev),
    ("POST", "/dev/dashboard/sql/{id}/explain",       "web::explain",      Access::Dev),
    ("GET",  "/dev/dashboard/logs",                   "web::logs_page",    Access::Dev),
    ("GET",  "/dev/dashboard/logs/stream",            "web::logs_stream",  Access::Dev),
    ("GET",  "/dev/dashboard/schema",                 "web::schema_page",  Access::Dev),
    ("GET",  "/dev/dashboard/schema/erd.svg",         "web::erd_svg",      Access::Dev),
    ("GET",  "/dev/dashboard/schema/erd.mmd",         "web::erd_mermaid",  Access::Dev),
    ("GET",  "/dev/dashboard/schema/tables/{name}",   "web::schema_table", Access::Dev),
    ("GET",  "/dev/dashboard/schema/migrations",      "web::migrations_page", Access::Dev),
    ("GET",  "/dev/dashboard/ai",                     "ai_web::ai_page",   Access::Dev),
    ("GET",  "/dev/dashboard/ai/panel",               "ai_web::ai_panel",  Access::Dev),
    ("POST", "/dev/dashboard/ai/ask",                 "ai_web::ai_ask",    Access::Dev),
    ("POST", "/dev/dashboard/ai/runs",                "ai_web::ai_runs",   Access::Dev),
    ("GET",  "/dev/dashboard/ai/runs/{id}",           "ai_web::run_page",  Access::Dev),
    ("GET",  "/dev/dashboard/ai/runs/{id}/stream",    "ai_web::run_stream", Access::Dev),
    ("POST", "/dev/dashboard/ai/runs/{id}/continue", "ai_web::run_continue", Access::Dev),
    ("GET",  "/dev/dashboard/ai/history",             "ai_web::history_page", Access::Dev),
    ("GET",  "/dev/dashboard/explain",                "ai_web::explain_page", Access::Dev),
    ("GET",  "/dev/dashboard/tests",                  "web::tests_page",   Access::Dev),
    ("POST", "/dev/dashboard/tests/run",              "web::tests_run",    Access::Dev),
    ("GET",  "/dev/dashboard/github",                 "web::github_page",  Access::Dev),
    ("GET",  "/dev/dashboard/lab",                    "web::lab_page",     Access::Dev),
    ("POST", "/dev/dashboard/lab/seed",               "web::lab_seed",     Access::Dev),
    ("POST", "/dev/dashboard/lab/clear",              "web::lab_clear",    Access::Dev),
    ("POST", "/dev/dashboard/lab/traffic",            "web::lab_traffic",  Access::Dev),
    ("POST", "/dev/dashboard/lab/stop",               "web::lab_stop",     Access::Dev),
    ("POST", "/dev/dashboard/lab/scenario",           "web::lab_scenario", Access::Dev),
    ("POST", "/dev/dashboard/lab/upstreams",          "web::lab_upstreams", Access::Dev),
    ("POST", "/dev/dashboard/lab/attack",             "web::lab_attack",   Access::Dev),
    ("GET",  "/dev/dashboard/config",                 "web::config_page",  Access::Dev),
    ("GET",  "/dev/dashboard/stream",                 "web::stream",       Access::Dev),
    ("POST", "/dev/dashboard/reset",                  "web::reset",        Access::Dev),
    ("POST", "/dev/csp-reports",                      "web::csp_report",   Access::Dev),
    ("GET",  "/dev/lab/orders",                       "lab::orders",       Access::Dev),
    ("GET",  "/dev/lab/orders/n-plus-one",            "lab::orders_n_plus_one", Access::Dev),
    ("GET",  "/dev/lab/slow-query",                   "lab::slow_query",   Access::Dev),
    ("GET",  "/dev/lab/slow",                         "lab::slow",         Access::Dev),
    ("GET",  "/dev/lab/panic",                        "lab::panic",        Access::Dev),
    ("GET",  "/dev/lab/error",                        "lab::error",        Access::Dev),
    ("GET",  "/dev/lab/checkout",                     "lab::checkout",     Access::Dev),
    ("GET",  "/dev/lab/upstream/{name}/{action}",     "lab::upstream",     Access::Dev),
    ("POST", "/dev/lab/upstream/{name}/{action}",     "lab::upstream",     Access::Dev),
    ("GET",  "/dev/lab/upstream/{name}/{action}/{id}", "lab::upstream_item", Access::Dev),
    ("GET",  "/dev/lab/sign-in",                      "lab::sign_in_form", Access::Dev),
    ("POST", "/dev/lab/sign-in",                      "lab::sign_in",      Access::Dev),
    ("GET",  "/dev/lab/ux",                           "lab::ux",           Access::Dev),
    ("GET",  "/dev/lab/ux/widget",                    "lab::ux_widget",    Access::Dev),
    ("GET",  "/dev/lab/ux/slow",                      "lab::ux_slow",      Access::Dev),
    ("GET",  "/admin/dashboard",                      "web::overview",     Access::Admin),
    ("GET",  "/admin/dashboard/charts",               "web::charts_page",  Access::Admin),
    ("GET",  "/admin/dashboard/requests",             "web::requests_page", Access::Admin),
    ("GET",  "/admin/dashboard/traces",               "traces_web::explorer", Access::Admin),
    ("GET",  "/admin/dashboard/traces/compare",       "traces_web::compare", Access::Admin),
    ("GET",  "/admin/dashboard/traces/lookup",        "traces_web::lookup", Access::Admin),
    ("GET",  "/admin/dashboard/traces/{id}",          "traces_web::trace_page", Access::Admin),
    ("GET",  "/admin/dashboard/traces/{id}/waterfall", "traces_web::waterfall_fragment", Access::Admin),
    ("GET",  "/admin/dashboard/traces/{id}/json",     "traces_web::trace_json", Access::Admin),
    ("GET",  "/admin/dashboard/methods",              "traces_web::methods_page", Access::Admin),
    ("GET",  "/admin/dashboard/upstreams",            "upstreams::upstreams_page", Access::Admin),
    ("GET",  "/admin/dashboard/security",             "threats::security_page", Access::Admin),
    ("GET",  "/admin/dashboard/frontend",             "rum::frontend_page", Access::Admin),
    ("GET",  "/admin/dashboard/sql",                  "web::sql_page",     Access::Admin),
    ("GET",  "/admin/dashboard/sql/{id}",             "web::sql_show",     Access::Admin),
    ("GET",  "/admin/dashboard/config",               "web::config_page",  Access::Admin),
    ("GET",  "/admin/dashboard/stream",               "web::stream",       Access::Admin),
    ("GET",  "/admin/dashboard/explain",              "ai_web::explain_page", Access::Admin),
    ("GET",  "/admin/dashboard/ai",                   "ai_web::ai_page",   Access::Admin),
    ("GET",  "/admin/dashboard/ai/panel",             "ai_web::ai_panel",  Access::Admin),
    ("POST", "/admin/dashboard/ai/ask",               "ai_web::ai_ask",    Access::Admin),
    ("POST", "/admin/dashboard/ai/runs",              "ai_web::ai_runs",   Access::Admin),
    ("GET",  "/admin/dashboard/ai/runs/{id}",         "ai_web::run_page",  Access::Admin),
    ("GET",  "/admin/dashboard/ai/runs/{id}/stream",  "ai_web::run_stream", Access::Admin),
    ("POST", "/admin/dashboard/ai/runs/{id}/continue", "ai_web::run_continue", Access::Admin),
    ("GET",  "/admin/dashboard/ai/history",           "ai_web::history_page", Access::Admin),
    ("GET",  "/dev/mailbox",                                "mailbox_web::index",         Access::Dev),
    ("GET",  "/dev/mailbox.json",                           "mailbox_web::list_json",     Access::Dev),
    ("GET",  "/dev/mailbox/latest.json",                    "mailbox_web::latest_json",   Access::Dev),
    ("GET",  "/dev/mailbox/list",                           "mailbox_web::list_fragment", Access::Dev),
    ("GET",  "/dev/mailbox/events",                         "mailbox_web::events",        Access::Dev),
    ("POST", "/dev/mailbox/clear",                          "mailbox_web::clear",         Access::Dev),
    ("POST", "/dev/mailbox/test",                           "mailbox_web::send_test",     Access::Dev),
    ("GET",  "/dev/mailbox/previews",                       "mailbox_web::previews_index", Access::Dev),
    ("GET",  "/dev/mailbox/previews/{mailer}/{action}",     "mailbox_web::preview_show",  Access::Dev),
    ("GET",  "/dev/mailbox/previews/{mailer}/{action}/html", "mailbox_web::preview_html", Access::Dev),
    ("GET",  "/dev/mailbox/{id}",                           "mailbox_web::show",          Access::Dev),
    ("GET",  "/dev/mailbox/{id}/html",                      "mailbox_web::html_body",     Access::Dev),
    ("GET",  "/dev/mailbox/{id}/message.eml",               "mailbox_web::download",      Access::Dev),
    ("GET",  "/dev/mailbox/{id}/attachments/{n}",           "mailbox_web::attachment",    Access::Dev),
    ("POST", "/dev/mailbox/{id}/delete",                    "mailbox_web::delete",        Access::Dev),
    ("GET",  "/dev/mailers",                                "mailbox_web::mailers_redirect", Access::Dev),
    ("GET",  "/session/new",                       "sessions::new",                Access::Public),
    ("POST", "/session/delete",                    "sessions::destroy",            Access::User),
    ("GET",  "/account",                           "account::show",                Access::User),
    ("GET",  "/account/confirm",                   "account::confirm",             Access::User),
    ("POST", "/account/sessions/delete-others",    "account::revoke_others",       Access::User),
    ("POST", "/account/sessions/{id}/delete",      "account::revoke",              Access::User),
    ("GET",  "/account/email/edit",                "account::edit_email",          Access::User),
    ("POST", "/account/email",                     "account::update_email",        Access::User),
    ("GET",  "/account/email/confirm/{token}",     "account::show_email_change",   Access::Public),
    ("POST", "/account/email/confirm/{token}",     "account::confirm_email_change", Access::Public),
    ("GET",  "/security/not-me/{token}",           "security::show",               Access::Public),
    ("POST", "/security/not-me/{token}",           "security::create",             Access::Public),
    ("POST", "/login_links",                       "login_links::create",          Access::Public),
    ("GET",  "/login_links/sent",                  "login_links::sent",            Access::Public),
    ("POST", "/login_links/code",                  "login_links::code",            Access::Public),
    ("GET",  "/login_links/{token}",               "login_links::show",            Access::Public),
    ("POST", "/login_links/{token}",               "login_links::consume",         Access::Public),
    ("POST", "/account/confirm/email",             "login_links::send_reauth_code", Access::User),
    ("POST", "/account/confirm/email/code",        "login_links::confirm_reauth_code", Access::User),
    ("GET",  "/admin/jobs",                         "jobs_web::overview",     Access::Admin),
    ("GET",  "/admin/jobs/list",                    "jobs_web::list",         Access::Admin),
    ("GET",  "/admin/jobs/errors",                  "jobs_web::errors",       Access::Admin),
    ("GET",  "/admin/jobs/cron",                    "jobs_web::cron",         Access::Admin),
    ("POST", "/admin/jobs/cron/{name}/run",         "jobs_web::cron_run",     Access::Admin),
    ("POST", "/admin/jobs/bulk",                    "jobs_web::bulk",         Access::Admin),
    ("POST", "/admin/jobs/queues/pause-all",        "jobs_web::pause_all",    Access::Admin),
    ("POST", "/admin/jobs/queues/resume-all",       "jobs_web::resume_all",   Access::Admin),
    ("POST", "/admin/jobs/queues/{name}/{action}",  "jobs_web::queue_action", Access::Admin),
    ("GET",  "/admin/jobs/{id}",                    "jobs_web::show",         Access::Admin),
    ("POST", "/admin/jobs/{id}/{action}",           "jobs_web::job_action",   Access::Admin),
    // samples: routes table
];

/// How many rows the tables hold (compile time).
const fn count(tables: &[&[Route]]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < tables.len() {
        n += tables[i].len();
        i += 1;
    }
    n
}

/// The tables, one after the other (compile time: `ROUTES` stays a plain `&[Route]`).
const fn concat<const N: usize>(tables: &[&[Route]]) -> [Route; N] {
    let mut out: [Route; N] = [("", "", "", Access::Public); N];
    let (mut k, mut i) = (0, 0);
    while i < tables.len() {
        let mut j = 0;
        while j < tables[i].len() {
            out[k] = tables[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    out
}
