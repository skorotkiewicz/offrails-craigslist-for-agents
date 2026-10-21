//! `AppState`: everything a handler or job needs, cheap to clone.

use std::sync::Arc;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::kernel::assets::{AssetCatalog, Assets};
use crate::kernel::cache::Cache;
use crate::kernel::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DatabaseConnection,
    pub cache: Cache,
    pub assets: AssetCatalog,
    pub rate_limit: RateLimiter,
    pub mailer: crate::kernel::mailer::Mailer,
    /// Auth emails sent after the response (auth-core): no request waits on SMTP.
    pub background: crate::kernel::background::Background,
    // samples: state fields
}

impl AppState {
    /// `TestApp` builds state the same way, with `Config::test()` and its own database.
    pub fn new(config: Config, db: DatabaseConnection) -> anyhow::Result<Self> {
        // WHY: uptime counts from boot. `BOOTED_AT` is lazy, so without this it started at the
        // first read (the first /dev/system view), which then showed "0ms".
        std::sync::LazyLock::force(&crate::kernel::ui::BOOTED_AT);
        let assets = AssetCatalog::load("static", config.is_development());
        // The dashboard's SQL callback, before the connection is cloned into the state.
        let db = crate::kernel::dashboard::instrument(db, &config);
        let mailer = select_mailer(&config, &db)?;
        // samples: state build
        Ok(Self {
            cache: Cache::new(db.clone()),
            rate_limit: RateLimiter::new(config.env),
            assets,
            mailer,
            background: crate::kernel::background::Background::default(),
            // samples: state init
            config: Arc::new(config),
            db,
        })
    }
}

/// Which mailer this environment uses.
/// GUIDE: .offrails/guides/mailers.md#kernel-spec-srckernelmailerrs
/// INVARIANT: development is always the dev mailbox unless MAILER says otherwise, even with SMTP_URL set (test: development_always_uses_the_dev_mailbox_even_with_smtp_url_set).
pub fn select_mailer(
    c: &Config,
    db: &DatabaseConnection,
) -> anyhow::Result<crate::kernel::mailer::Mailer> {
    use crate::config::AppEnv;
    use crate::kernel::mailer::Mailer;

    let from: lettre::message::Mailbox = c
        .mail
        .from
        .parse()
        .map_err(|_| anyhow::anyhow!("MAIL_FROM is not a valid address: {:?}", c.mail.from))?;
    let smtp = |url: Option<&str>| match url {
        Some(url) => Mailer::smtp(url, from.clone()),
        None => anyhow::bail!("MAILER=smtp needs SMTP_URL"),
    };
    Ok(match (c.env, c.mail.mailer.as_deref()) {
        (AppEnv::Test, _) => Mailer::memory(),
        (_, Some("smtp")) => smtp(c.mail.smtp_url.as_deref())?,
        (_, Some("local")) => Mailer::local(db.clone(), from),
        (_, Some("log")) => Mailer::Log,
        (_, Some(other)) => anyhow::bail!("MAILER must be smtp, local or log (got {other:?})"),
        (AppEnv::Development, None) => Mailer::local(db.clone(), from),
        (AppEnv::Production, None) => match c.mail.smtp_url.as_deref() {
            Some(url) => smtp(Some(url))?,
            None => {
                tracing::warn!("SMTP_URL is not set: email is logged, not sent");
                Mailer::Log
            }
        },
    })
}
// samples: state fns

// The kernel's routers are generic over any state that can hand out these.
impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Assets {
    fn from_ref(state: &AppState) -> Self {
        state.assets.assets()
    }
}
