//! Typed configuration, parsed from env vars once at startup.
//! This is the only module that reads environment variables.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

/// The human product name: page titles, kernel UIs (/dev), the default MAIL_FROM. It is
/// brand.toml's `name` (`cargo offrails brand` writes src/brand.rs). Never derived from the crate
/// name: crates get renamed, products don't.
pub const APP_NAME: &str = crate::brand::NAME;
/// The tagline: brand.toml's `tagline` (`cargo offrails brand` writes src/brand.rs), the one
/// source for the home page title ("Board: <tagline>"), the social card and any nav that
/// shows it. Change it in brand.toml, never here.
pub const APP_TAGLINE: &str = crate::brand::TAGLINE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Test,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

/// `APP_TLS_CERT` + `APP_TLS_KEY`: serve HTTPS (h2 + http/1.1 via ALPN) directly.
#[derive(Debug, Clone)]
pub struct TlsFiles {
    pub cert: String,
    pub key: String,
}

#[derive(Clone)]
pub struct Config {
    pub env: AppEnv,
    pub database_url: String,
    /// `DATABASE_POOL`: 4 for SQLite (one writer at a time; each connection keeps its own page
    /// cache), 10 for Postgres.
    pub database_pool: u32,
    /// `APP_WORKER_THREADS`: Tokio worker threads (default 2). Raise it for CPU-bound request work
    /// on a box with more cores; I/O-bound apps rarely gain past 2–4.
    pub worker_threads: usize,
    /// `APP_BLOCKING_THREADS`: the cap on Tokio's blocking pool (default 16; Tokio's own is 512).
    /// `spawn_blocking` work (static files, password hashing) queues past it instead of adding threads.
    pub blocking_threads: usize,
    pub app_url: String,
    pub port: u16,
    /// `APP_SHUTDOWN_DRAIN_SECS`: how long open connections get after SIGTERM.
    pub shutdown_drain: Duration,
    /// `APP_TRUSTED_PROXY`: read the client IP from `X-Forwarded-For` (rate limiting).
    pub trusted_proxy: bool,
    /// `APP_TRUSTED_PROXY_HOPS` (default 1): how many proxies append to `X-Forwarded-For` before
    /// the app. The client is that many hops from the right: 1 behind one platform proxy (Fly, Render),
    /// 2 behind Cloudflare → Caddy → app. Fewer than the real count takes a proxy's address
    /// (every user shares one rate-limit key); more lets a client forge its address.
    pub trusted_proxy_hops: u8,
    pub log_format: LogFormat,
    /// `APP_REVISION`: git sha of the build, shown on /dev/system.
    pub revision: Option<String>,
    pub tls: Option<TlsFiles>,
    /// `APP_DEV_HTTP_PORT` (development only, with TLS on): the same app on plain http.
    pub dev_http_port: Option<u16>,
    /// `APP_BIND`: the address `serve` listens on. Development: 127.0.0.1 (this machine only);
    /// production: 0.0.0.0 (the container's interface, behind the platform's proxy).
    pub bind: IpAddr,
    /// `APP_ALLOWED_HOSTS`: extra `Host` names development answers (a tunnel, a LAN name);
    /// `.example.com` allows its subdomains too. Localhost and `APP_URL`'s host always work.
    pub allowed_hosts: Vec<String>,
    /// Kept for parts that read it: always `false` now that an unset `APP_ENV` refuses to boot.
    pub env_defaulted: bool,
    /// The dev dashboard: production mode, its admin gate, GitHub, the test runner.
    pub dashboard: DashboardConfig,
    pub mail: MailConfig,
    pub auth: AuthConfig,
    pub magic_link: MagicLinkConfig,
    /// The job queue: `APP_QUEUES`, `APP_INLINE_WORKER`, `APP_JOBS_*`.
    pub jobs: JobsConfig,
    /// The jobs dashboard's admin gate: `APP_ADMIN_PASSWORD`.
    pub jobs_dashboard: JobsDashboardConfig,
    // samples: config fields
}

// Credentials live in these fields, so Debug is hand-written.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Config");
        d.field("env", &self.env)
            .field("database_url", &"[redacted]")
            .field("database_pool", &self.database_pool)
            .field("worker_threads", &self.worker_threads)
            .field("blocking_threads", &self.blocking_threads)
            .field("app_url", &self.app_url)
            .field("port", &self.port)
            .field("shutdown_drain", &self.shutdown_drain)
            .field("trusted_proxy", &self.trusted_proxy)
            .field("trusted_proxy_hops", &self.trusted_proxy_hops)
            .field("log_format", &self.log_format)
            .field("revision", &self.revision)
            .field("tls", &self.tls.is_some())
            .field("dev_http_port", &self.dev_http_port)
            .field("bind", &self.bind)
            .field("allowed_hosts", &self.allowed_hosts)
            .field("env_defaulted", &self.env_defaulted);
        d.field("dashboard", &self.dashboard);
        d.field("mail", &self.mail);
        d.field("auth", &self.auth);
        d.field("magic_link", &self.magic_link);
        d.field("jobs", &self.jobs);
        d.field("jobs_dashboard", &self.jobs_dashboard);
        // samples: config debug
        d.finish()
    }
}

const DEV_DATABASE_URL: &str = "sqlite://storage/development.sqlite3?mode=rwc";

/// In development `APP_URL` follows `PORT` (https when `bin/dev` serves TLS), so links and the
/// CSRF origin check still work when the app runs on another port.
pub fn dev_app_url(port: u16, https: bool) -> String {
    let scheme = if https { "https" } else { "http" };
    format!("{scheme}://localhost:{port}")
}

/// Where links (emails, canonical URLs, the CSRF origin) point. Production: `APP_URL`, required.
/// Development: the address actually served. A localhost `APP_URL` left in `.env` from another
/// port would send every emailed link to the wrong server, so on localhost the port and scheme
/// always follow `PORT` and TLS; a non-local `APP_URL` (a tunnel) is kept as given.
pub fn resolve_app_url(env: AppEnv, app_url: Option<&str>, port: u16, https: bool) -> String {
    let given = app_url.map(|u| u.trim().trim_end_matches('/').to_owned());
    match (env, given) {
        (AppEnv::Production, Some(url)) => url,
        (AppEnv::Production, None) => String::new(),
        (_, Some(url)) if !is_localhost(&url) => url,
        (_, Some(url)) => {
            let served = dev_app_url(port, https);
            if url != served {
                tracing::warn!(app_url = %url, served = %served, "APP_URL points at another local address; links follow the one being served");
            }
            served
        }
        (_, None) => dev_app_url(port, https),
    }
}

/// `APP_BIND`, or the environment's default: loopback in development, every interface otherwise.
// SECURITY: development serves /dev tools (mailbox, dashboard, SQL, logs) without a login; on
// 0.0.0.0 anyone on the café Wi-Fi could open them. A container sets APP_BIND=0.0.0.0 explicitly
// (test: development_binds_loopback_unless_told_otherwise).
pub fn bind_address(env: AppEnv, raw: Option<&str>) -> Result<IpAddr, String> {
    match raw.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => v
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(|_| {
                format!("APP_BIND (expected an IP address such as 127.0.0.1 or 0.0.0.0, got {v:?})")
            }),
        None if env == AppEnv::Development => Ok(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        None => Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
    }
}

/// `APP_ALLOWED_HOSTS`: comma-separated host names, lowercased, ports dropped.
pub fn parse_allowed_hosts(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|h| h.trim().to_ascii_lowercase())
        .filter(|h| !h.is_empty())
        .collect()
}

fn is_localhost(url: &str) -> bool {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split('/').next().unwrap_or_default();
    let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "0.0.0.0")
}

impl Config {
    /// Called once in `main`. Collects ALL missing or invalid vars and fails with one message.
    pub fn from_env() -> anyhow::Result<Self> {
        let get_env = || {
            std::env::var("APP_ENV")
                .ok()
                .filter(|v| !v.trim().is_empty())
        };
        // SECURITY: an unset APP_ENV is an error, never a default. Development mounts /dev tools
        // without a login, so "forgot to set it on a server" must not silently become
        // development. `.env` (bin/setup writes APP_ENV=development there) is read first, so
        // `cargo run` in a set-up checkout still works; a production image has no `.env`
        // (.dockerignore) and sets APP_ENV itself (test: an_unset_app_env_refuses_to_boot, kernel_test.rs).
        if get_env().is_none() {
            dotenvy::dotenv().ok();
        }
        let env = match get_env().as_deref() {
            Some("development") => AppEnv::Development,
            Some("test") => AppEnv::Test,
            Some("production") => AppEnv::Production,
            Some(other) => {
                anyhow::bail!("invalid APP_ENV: {other} (development | test | production)")
            }
            None => anyhow::bail!(
                "APP_ENV is not set. Locally: APP_ENV=development in .env (bin/setup creates it \
                 from .env.example). On every server: APP_ENV=production (the Dockerfile sets it)."
            ),
        };
        let env_defaulted = false;

        let get = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let mut missing: Vec<String> = Vec::new();
        let mut invalid: Vec<String> = Vec::new();
        let production = env == AppEnv::Production;

        let mut parse_num = |name: &str, default: u64| -> u64 {
            match get(name) {
                None => default,
                Some(v) => v.parse().unwrap_or_else(|_| {
                    invalid.push(format!("{name} (expected a number)"));
                    default
                }),
            }
        };
        // PORT first: the development APP_URL default depends on it.
        let port = parse_num("PORT", 3000) as u16;
        // 0 = the backend's default, chosen below once DATABASE_URL is known.
        let database_pool = parse_num("DATABASE_POOL", 0) as u32;
        let worker_threads = (parse_num("APP_WORKER_THREADS", 2) as usize).max(1);
        let blocking_threads = (parse_num("APP_BLOCKING_THREADS", 16) as usize).max(1);
        let shutdown_drain = Duration::from_secs(parse_num("APP_SHUTDOWN_DRAIN_SECS", 5));
        let dev_http_port = get("APP_DEV_HTTP_PORT").map(|v| {
            v.parse::<u16>().unwrap_or_else(|_| {
                invalid.push("APP_DEV_HTTP_PORT (expected a port number)".into());
                0
            })
        });
        if dev_http_port.is_some() && env != AppEnv::Development {
            invalid.push("APP_DEV_HTTP_PORT (development only)".into());
        }

        let tls = match (get("APP_TLS_CERT"), get("APP_TLS_KEY")) {
            (Some(cert), Some(key)) => Some(TlsFiles { cert, key }),
            (None, None) => None,
            _ => {
                invalid.push("APP_TLS_CERT and APP_TLS_KEY (set both, or neither)".into());
                None
            }
        };

        let database_url = match get("DATABASE_URL") {
            Some(v) => v,
            None if production => {
                missing.push("DATABASE_URL".into());
                String::new()
            }
            None => DEV_DATABASE_URL.into(),
        };
        let database_pool = match database_pool {
            0 if database_url.starts_with("sqlite:") => 4,
            0 => 10,
            n => n,
        };
        if production && get("APP_URL").is_none() {
            missing.push("APP_URL".into());
        }
        // With APP_DEV_HTTP_PORT the browser uses the plain-http port; links stay http.
        let https = tls.is_some() && dev_http_port.is_none();
        let app_url = resolve_app_url(env, get("APP_URL").as_deref(), port, https);

        let trusted_proxy = match get("APP_TRUSTED_PROXY").as_deref() {
            None | Some("false") | Some("0") => false,
            Some("true") | Some("1") => true,
            Some(_) => {
                invalid.push("APP_TRUSTED_PROXY (expected true or false)".into());
                false
            }
        };

        let trusted_proxy_hops = match get("APP_TRUSTED_PROXY_HOPS") {
            None => 1,
            Some(v) => match v.parse::<u8>() {
                Ok(n) if n >= 1 => n,
                _ => {
                    invalid.push("APP_TRUSTED_PROXY_HOPS (expected a number from 1 to 255)".into());
                    1
                }
            },
        };

        let bind = bind_address(env, get("APP_BIND").as_deref()).unwrap_or_else(|problem| {
            invalid.push(problem);
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        });
        let allowed_hosts = parse_allowed_hosts(get("APP_ALLOWED_HOSTS").as_deref());

        let log_format = match get("LOG_FORMAT").as_deref() {
            None if production => LogFormat::Json,
            None => LogFormat::Pretty,
            Some("json") => LogFormat::Json,
            Some("pretty") => LogFormat::Pretty,
            Some(_) => {
                invalid.push("LOG_FORMAT (expected pretty or json)".into());
                LogFormat::Pretty
            }
        };

        let dashboard = DashboardConfig::from_env(env)?;
        let mail = MailConfig::from_env(env)?;
        let auth = AuthConfig::from_env(env, &mut missing, &mut invalid);
        let magic_link = MagicLinkConfig::from_env(&mut invalid);
        let jobs = JobsConfig::from_env(env)?;
        let jobs_dashboard = JobsDashboardConfig::from_env()?;
        // samples: config parse

        if !missing.is_empty() || !invalid.is_empty() {
            let mut parts = Vec::new();
            if !missing.is_empty() {
                parts.push(format!("missing required env vars: {}", missing.join(", ")));
            }
            if !invalid.is_empty() {
                parts.push(format!("invalid env vars: {}", invalid.join(", ")));
            }
            anyhow::bail!("{} (see .env.example)", parts.join("; "));
        }

        Ok(Self {
            env,
            database_url,
            database_pool,
            worker_threads,
            blocking_threads,
            app_url,
            port,
            shutdown_drain,
            trusted_proxy,
            trusted_proxy_hops,
            log_format,
            revision: get("APP_REVISION"),
            tls,
            dev_http_port,
            bind,
            allowed_hosts,
            env_defaulted,
            dashboard,
            mail,
            auth,
            magic_link,
            jobs,
            jobs_dashboard,
            // samples: config init
        })
    }

    /// The test suite's config. Never reads the environment.
    pub fn test() -> Self {
        Self {
            env: AppEnv::Test,
            database_url: "sqlite::memory:".into(),
            database_pool: 5,
            worker_threads: 2,
            blocking_threads: 16,
            app_url: dev_app_url(3000, false),
            port: 3000,
            shutdown_drain: Duration::from_secs(5),
            trusted_proxy: false,
            trusted_proxy_hops: 1,
            log_format: LogFormat::Pretty,
            revision: None,
            tls: None,
            dev_http_port: None,
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            allowed_hosts: Vec::new(),
            env_defaulted: false,
            dashboard: DashboardConfig::for_tests(),
            mail: MailConfig::test(),
            auth: AuthConfig::test(),
            magic_link: MagicLinkConfig::default(),
            jobs: JobsConfig::test(),
            jobs_dashboard: JobsDashboardConfig::default(),
            // samples: config test
        }
    }

    pub fn is_production(&self) -> bool {
        self.env == AppEnv::Production
    }

    pub fn is_development(&self) -> bool {
        self.env == AppEnv::Development
    }

    /// Loud warnings `main` logs right after logging starts (parts add theirs here).
    pub fn boot_warnings(&self) -> Vec<String> {
        Vec::new()
    }
}

/// The dev dashboard (`kernel/dashboard`).
#[derive(Clone)]
pub struct DashboardConfig {
    /// `DASHBOARD_IN_PRODUCTION=true`: mount the read-only `/admin/dashboard` in production
    /// (it also needs an admin gate: `APP_ADMIN_PASSWORD` or admin accounts).
    pub in_production: bool,
    /// `APP_ADMIN_PASSWORD`: HTTP Basic password for `/admin/dashboard` without admin accounts.
    pub admin_password: Option<String>,
    /// `GITHUB_REPO=owner/name`: the repo the GitHub page shows.
    pub github_repo: Option<String>,
    /// `GITHUB_TOKEN`: optional, read-only. Never shown or logged.
    pub github_token: Option<String>,
    /// `GITHUB_API_URL` (GitHub Enterprise); `https://api.github.com` by default.
    pub github_api: String,
    /// `DASHBOARD_GITHUB_FIXTURE=dir`: recorded API answers instead of the network (development
    /// only: the demo and its screenshots).
    pub github_fixture: Option<String>,
    /// `TEST_DATABASE_URL`: where the Run tests button runs the Postgres suite.
    pub test_database_url: Option<String>,
    /// `CARGO_TARGET_DIR`: where nextest writes its JUnit report.
    pub cargo_target_dir: Option<String>,
    /// `DASHBOARD_DEMO_HISTORY=true` (development only): fill the last hour with synthetic,
    /// clearly marked history at boot (the demo and its screenshots).
    pub demo_history: bool,
    /// `DASHBOARD_HISTORY=false`: keep the charts' history and the notable traces in memory only
    /// (on by default: `storage/dashboard/history.sqlite3`, compressed, survives restarts).
    pub history: bool,
    /// `DASHBOARD_HISTORY_PATH`: where the history file lives (in `storage_dir` by default).
    pub history_path: String,
    /// `DASHBOARD_STORAGE_DIR`: the dashboard's own files (the history file, past Ask AI runs);
    /// `storage/dashboard` by default. Tests get one per process (`for_tests`).
    pub storage_dir: String,
    /// `DASHBOARD_PROFILER=false`: no CPU sampling at all (development only, Linux and macOS:
    /// the Hot functions page starts and stops it); `always`: sampling from boot.
    pub profiler: bool,
    pub profiler_at_boot: bool,
    /// `DASHBOARD_RUM=true|false`: the browser beacon (rage and dead clicks, JS errors, Web
    /// Vitals). Unset: on in development, off in production (it also needs the read-only view).
    pub rum: Option<bool>,
    /// `DASHBOARD_RUM_SAMPLE=0.1`: the share of page views that load the beacon (0 to 1; unset:
    /// all in development, 0.1 in production).
    pub rum_sample: Option<f64>,
    /// The optional "Ask AI" assistant (`DASHBOARD_AI_*`).
    pub ai: DashboardAi,
}

/// The dashboard's folder for a config that didn't come from the environment (tests): one per
/// process, so tests never share past runs or history with each other or with the app.
// WHY the time too: a process id is reused, and a folder left by an earlier run would hand this
// process another one's files.
fn test_storage() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        std::env::temp_dir().join(format!("offrails-dashboard-{}-{nanos}", std::process::id()))
    })
    .clone()
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            in_production: false,
            admin_password: None,
            github_repo: None,
            github_token: None,
            github_api: "https://api.github.com".to_owned(),
            github_fixture: None,
            test_database_url: None,
            cargo_target_dir: None,
            demo_history: false,
            history: true,
            history_path: test_storage().join("history.sqlite3").display().to_string(),
            storage_dir: test_storage().display().to_string(),
            profiler: true,
            profiler_at_boot: false,
            rum: None,
            rum_sample: None,
            ai: DashboardAi::default(),
        }
    }
}

impl DashboardConfig {
    /// The defaults (tests): the dashboard's files in a folder of this process's own.
    pub fn for_tests() -> Self {
        Self::default()
    }

    pub fn from_env(env: AppEnv) -> anyhow::Result<Self> {
        let get = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let mut invalid = Vec::new();
        let in_production = match get("DASHBOARD_IN_PRODUCTION").as_deref() {
            None | Some("false") | Some("0") => false,
            Some("true") | Some("1") => true,
            Some(_) => {
                invalid.push("DASHBOARD_IN_PRODUCTION (expected true or false)");
                false
            }
        };
        let admin_password = get("APP_ADMIN_PASSWORD");
        if !admin_password_ok(admin_password.as_deref()) {
            invalid.push("APP_ADMIN_PASSWORD (at least 16 characters)");
        }
        let github_repo = get("GITHUB_REPO");
        if github_repo.as_deref().is_some_and(|r| !valid_repo(r)) {
            invalid.push("GITHUB_REPO (expected owner/name)");
        }
        let github_api =
            get("GITHUB_API_URL").unwrap_or_else(|| "https://api.github.com".to_owned());
        if !github_api.starts_with("https://") {
            invalid.push("GITHUB_API_URL (expected https://)");
        }
        let history = match get("DASHBOARD_HISTORY").as_deref() {
            None | Some("true") | Some("1") => true,
            Some("false") | Some("0") => false,
            Some(_) => {
                invalid.push("DASHBOARD_HISTORY (expected true or false)");
                true
            }
        };
        let (profiler, profiler_at_boot) = match get("DASHBOARD_PROFILER").as_deref() {
            None | Some("true") | Some("1") => (true, false),
            Some("always") => (true, true),
            Some("false") | Some("0") => (false, false),
            Some(_) => {
                invalid.push("DASHBOARD_PROFILER (expected true, always or false)");
                (true, false)
            }
        };
        let rum = match get("DASHBOARD_RUM").as_deref() {
            None => None,
            Some("true") | Some("1") => Some(true),
            Some("false") | Some("0") => Some(false),
            Some(_) => {
                invalid.push("DASHBOARD_RUM (expected true or false)");
                None
            }
        };
        let rum_sample = match get("DASHBOARD_RUM_SAMPLE") {
            None => None,
            Some(v) => match v.trim().parse::<f64>() {
                Ok(f) if (0.0..=1.0).contains(&f) => Some(f),
                _ => {
                    invalid.push("DASHBOARD_RUM_SAMPLE (a share of page views, 0 to 1)");
                    None
                }
            },
        };
        let ai = DashboardAi::from_env(&get, &mut invalid);
        if !invalid.is_empty() {
            anyhow::bail!(
                "invalid env vars: {} (see .env.example)",
                invalid.join(", ")
            );
        }
        Ok(Self {
            in_production,
            admin_password,
            github_repo,
            github_token: get("GITHUB_TOKEN"),
            github_api,
            github_fixture: get("DASHBOARD_GITHUB_FIXTURE").filter(|_| env == AppEnv::Development),
            test_database_url: get("TEST_DATABASE_URL"),
            cargo_target_dir: get("CARGO_TARGET_DIR"),
            demo_history: env == AppEnv::Development
                && matches!(get("DASHBOARD_DEMO_HISTORY").as_deref(), Some("true" | "1")),
            history,
            history_path: get("DASHBOARD_HISTORY_PATH").unwrap_or_else(|| {
                format!(
                    "{}/history.sqlite3",
                    get("DASHBOARD_STORAGE_DIR").unwrap_or_else(|| "storage/dashboard".to_owned())
                )
            }),
            storage_dir: get("DASHBOARD_STORAGE_DIR")
                .unwrap_or_else(|| "storage/dashboard".to_owned()),
            profiler,
            profiler_at_boot,
            rum,
            rum_sample,
            ai,
        })
    }
}

/// Which API shape the dashboard's assistant speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiProvider {
    /// Anthropic's Messages API (`POST /v1/messages`).
    Anthropic,
    /// Any OpenAI-compatible chat completions API: OpenAI, Ollama, LM Studio, a gateway.
    OpenAiCompatible,
}

/// The dashboard's optional "Ask AI" assistant: env only, never stored, never shown back.
#[derive(Clone)]
pub struct DashboardAi {
    /// `DASHBOARD_AI_PROVIDER=anthropic|openai-compatible`; unset: no assistant.
    pub provider: Option<AiProvider>,
    /// `DASHBOARD_AI_API_KEY`: required for Anthropic, optional for a local model. Never
    /// shown, logged or recorded.
    pub api_key: Option<String>,
    /// `DASHBOARD_AI_BASE_URL`: https, or http to a loopback address (a local model).
    pub base_url: String,
    /// `DASHBOARD_AI_MODEL`: `claude-sonnet-5-5` by default for Anthropic.
    pub model: String,
    /// `DASHBOARD_AI_IN_PRODUCTION=true`: also in the read-only `/admin/dashboard`, behind its gate.
    pub in_production: bool,
    /// `DASHBOARD_AI_PROACTIVE=true` (development only): the watcher posts incident cards and, with
    /// a provider, investigates each signal once (debounced, budgeted).
    pub proactive: bool,
    /// `DASHBOARD_AI_TOOLS=core`: offer the agent its small-model tool set (`ai_tools` `core`).
    pub tools_core: bool,
    /// `DASHBOARD_AI_REFERER`, `DASHBOARD_AI_TITLE`: OpenRouter's optional app attribution
    /// headers (`HTTP-Referer`, `X-Title`), sent to OpenAI-compatible providers.
    pub referer: Option<String>,
    pub title: Option<String>,
    /// `DASHBOARD_AI_DAILY_QUESTIONS`: runs a day (default `ai::DAILY_REQUESTS`).
    pub daily_questions: u32,
    /// `DASHBOARD_AI_DAILY_TOKENS`: tokens a day, in and out (default `ai::DAILY_TOKENS`).
    pub daily_tokens: u64,
    /// `DASHBOARD_AI_RUN_MAX_TOKENS`: tokens one run may use (default `ai::RUN_TOKENS`).
    pub run_max_tokens: u64,
    /// `DASHBOARD_AI_MAX_ANSWER_TOKENS`: one answer's length cap (`max_tokens`; default
    /// `ai::MAX_ANSWER_TOKENS`). A cut answer gets a Continue button.
    pub max_answer_tokens: u32,
    /// `DASHBOARD_AI_PRICE_IN`, `DASHBOARD_AI_PRICE_OUT`: the model's USD per million input and
    /// output tokens (from the provider's price list), for the "spent today" estimate.
    pub price_in: Option<f64>,
    pub price_out: Option<f64>,
    /// `DASHBOARD_AI_DAILY_USD`: stop for the day once the estimate reaches this (needs both prices).
    pub daily_usd: Option<f64>,
}

impl Default for DashboardAi {
    fn default() -> Self {
        Self {
            provider: None,
            api_key: None,
            base_url: String::new(),
            model: String::new(),
            in_production: false,
            proactive: false,
            tools_core: false,
            referer: None,
            title: None,
            daily_questions: crate::kernel::dashboard::ai::DAILY_REQUESTS,
            daily_tokens: crate::kernel::dashboard::ai::DAILY_TOKENS,
            run_max_tokens: crate::kernel::dashboard::ai::RUN_TOKENS,
            max_answer_tokens: crate::kernel::dashboard::ai::MAX_ANSWER_TOKENS,
            price_in: None,
            price_out: None,
            daily_usd: None,
        }
    }
}

/// A number from the environment within `lo..=hi`, or `default` when unset; `why` when invalid.
fn ai_number<T: std::str::FromStr + PartialOrd + Copy>(
    raw: Option<String>,
    (lo, hi): (T, T),
    default: T,
    invalid: &mut Vec<&'static str>,
    why: &'static str,
) -> T {
    match raw {
        None => default,
        Some(v) => match v.trim().replace('_', "").parse::<T>() {
            Ok(n) if n >= lo && n <= hi => n,
            _ => {
                invalid.push(why);
                default
            }
        },
    }
}

impl DashboardAi {
    /// Reads `DASHBOARD_AI_*` through `get` (the environment, or a test's map); each invalid
    /// variable is pushed to `invalid` with what it expects.
    pub fn from_env(get: &dyn Fn(&str) -> Option<String>, invalid: &mut Vec<&'static str>) -> Self {
        let provider = match get("DASHBOARD_AI_PROVIDER").as_deref() {
            None => None,
            Some("anthropic") => Some(AiProvider::Anthropic),
            Some("openai-compatible" | "openai") => Some(AiProvider::OpenAiCompatible),
            Some(_) => {
                invalid.push("DASHBOARD_AI_PROVIDER (expected anthropic or openai-compatible)");
                None
            }
        };
        let base_url = get("DASHBOARD_AI_BASE_URL").unwrap_or_else(|| match provider {
            Some(AiProvider::OpenAiCompatible) => "https://api.openai.com/v1".into(),
            _ => "https://api.anthropic.com".into(),
        });
        let api_key = get("DASHBOARD_AI_API_KEY");
        let model = get("DASHBOARD_AI_MODEL").unwrap_or_else(|| match provider {
            Some(AiProvider::Anthropic) => "claude-sonnet-5-5".into(),
            _ => String::new(),
        });
        if provider.is_some() {
            if !ai_base_url_ok(&base_url) {
                invalid.push(
                    "DASHBOARD_AI_BASE_URL (https://, or http:// to localhost/127.0.0.1/[::1]; no credentials, query or fragment)",
                );
            }
            if model.is_empty() {
                invalid.push("DASHBOARD_AI_MODEL (required for openai-compatible)");
            }
            if provider == Some(AiProvider::Anthropic) && api_key.is_none() {
                invalid.push("DASHBOARD_AI_API_KEY (required for anthropic)");
            }
        }
        let flag = |name: &str, invalid: &mut Vec<&'static str>, why: &'static str| match get(name)
            .as_deref()
        {
            None | Some("false") | Some("0") => false,
            Some("true") | Some("1") => true,
            Some(_) => {
                invalid.push(why);
                false
            }
        };
        let in_production = flag(
            "DASHBOARD_AI_IN_PRODUCTION",
            invalid,
            "DASHBOARD_AI_IN_PRODUCTION (expected true or false)",
        );
        let proactive = flag(
            "DASHBOARD_AI_PROACTIVE",
            invalid,
            "DASHBOARD_AI_PROACTIVE (expected true or false)",
        );
        let tools_core = match get("DASHBOARD_AI_TOOLS").as_deref() {
            None | Some("all") => false,
            Some("core") => true,
            Some(_) => {
                invalid.push("DASHBOARD_AI_TOOLS (expected all or core)");
                false
            }
        };
        let referer = get("DASHBOARD_AI_REFERER");
        if referer.as_deref().is_some_and(|r| !ai_referer_ok(r)) {
            invalid.push("DASHBOARD_AI_REFERER (an http(s) URL without credentials)");
        }
        let title = get("DASHBOARD_AI_TITLE");
        if title.as_deref().is_some_and(|t| !ai_title_ok(t)) {
            invalid.push("DASHBOARD_AI_TITLE (1 to 64 printable ASCII characters)");
        }
        let d = Self::default();
        let daily_questions = ai_number(
            get("DASHBOARD_AI_DAILY_QUESTIONS"),
            (1, 100_000),
            d.daily_questions,
            invalid,
            "DASHBOARD_AI_DAILY_QUESTIONS (a whole number, 1 to 100000)",
        );
        let daily_tokens = ai_number(
            get("DASHBOARD_AI_DAILY_TOKENS"),
            (1_000, 10_000_000_000),
            d.daily_tokens,
            invalid,
            "DASHBOARD_AI_DAILY_TOKENS (a whole number, 1000 to 10000000000)",
        );
        let run_max_tokens = ai_number(
            get("DASHBOARD_AI_RUN_MAX_TOKENS"),
            (1_000, 100_000_000),
            d.run_max_tokens,
            invalid,
            "DASHBOARD_AI_RUN_MAX_TOKENS (a whole number, 1000 to 100000000)",
        );
        let max_answer_tokens = ai_number(
            get("DASHBOARD_AI_MAX_ANSWER_TOKENS"),
            (256, 128_000),
            d.max_answer_tokens,
            invalid,
            "DASHBOARD_AI_MAX_ANSWER_TOKENS (a whole number, 256 to 128000)",
        );
        let price = |name: &str, why: &'static str, invalid: &mut Vec<&'static str>| {
            get(name)
                .map(|v| ai_number(Some(v), (0.0, 1_000.0), f64::NAN, invalid, why))
                .filter(|p| !p.is_nan())
        };
        let price_in = price(
            "DASHBOARD_AI_PRICE_IN",
            "DASHBOARD_AI_PRICE_IN (USD per million input tokens, 0 to 1000)",
            invalid,
        );
        let price_out = price(
            "DASHBOARD_AI_PRICE_OUT",
            "DASHBOARD_AI_PRICE_OUT (USD per million output tokens, 0 to 1000)",
            invalid,
        );
        let daily_usd = get("DASHBOARD_AI_DAILY_USD")
            .map(|v| {
                ai_number(
                    Some(v),
                    (0.01, 100_000.0),
                    f64::NAN,
                    invalid,
                    "DASHBOARD_AI_DAILY_USD (USD a day, 0.01 to 100000)",
                )
            })
            .filter(|u| !u.is_nan());
        if daily_usd.is_some() && (price_in.is_none() || price_out.is_none()) {
            invalid.push(
                "DASHBOARD_AI_DAILY_USD (needs DASHBOARD_AI_PRICE_IN and DASHBOARD_AI_PRICE_OUT to estimate the spend)",
            );
        }
        Self {
            provider,
            api_key,
            base_url,
            model,
            in_production,
            proactive,
            tools_core,
            referer,
            title,
            daily_questions,
            daily_tokens,
            run_max_tokens,
            max_answer_tokens,
            price_in,
            price_out,
            daily_usd,
        }
    }
}

/// `DASHBOARD_AI_REFERER`: a plain http(s) URL (it only names the app to OpenRouter).
pub fn ai_referer_ok(raw: &str) -> bool {
    reqwest::Url::parse(raw).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.username().is_empty()
            && u.password().is_none()
            && raw.len() <= 200
    })
}

/// `DASHBOARD_AI_TITLE`: a short header value.
pub fn ai_title_ok(raw: &str) -> bool {
    !raw.is_empty() && raw.len() <= 64 && raw.bytes().all(|b| (0x20..0x7f).contains(&b))
}

/// SECURITY: the assistant's address comes from the environment only, and is https unless it is
/// this machine (a local model): the dashboard's context never crosses a network in clear text,
/// and no URL carries credentials (test: the_ai_base_url_is_https_or_loopback).
pub fn ai_base_url_ok(raw: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(raw) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let host = url.host_str().unwrap_or_default();
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    match url.scheme() {
        "https" => !host.is_empty(),
        "http" => loopback,
        _ => false,
    }
}

impl fmt::Debug for DashboardAi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DashboardAi")
            .field("provider", &self.provider)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("in_production", &self.in_production)
            .field("proactive", &self.proactive)
            .field("tools_core", &self.tools_core)
            .field("referer", &self.referer)
            .field("title", &self.title)
            .field("daily_questions", &self.daily_questions)
            .field("daily_tokens", &self.daily_tokens)
            .field("run_max_tokens", &self.run_max_tokens)
            .field("max_answer_tokens", &self.max_answer_tokens)
            .field("price_in", &self.price_in)
            .field("price_out", &self.price_out)
            .field("daily_usd", &self.daily_usd)
            .finish()
    }
}

/// SECURITY: a short password behind HTTP Basic is guessable; the app refuses to boot with one
/// (test: the_production_dashboard_needs_the_admin_password).
pub fn admin_password_ok(password: Option<&str>) -> bool {
    password.is_none_or(|p| p.chars().count() >= 16)
}

/// `owner/name` with GitHub's characters only (it becomes part of an API path).
pub fn valid_repo(repo: &str) -> bool {
    let ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && s != "."
            && s != ".."
    };
    matches!(repo.split_once('/'), Some((owner, name)) if ok(owner) && ok(name))
}

impl fmt::Debug for DashboardConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let set = |v: &Option<String>| v.as_ref().map(|_| "[redacted]");
        f.debug_struct("DashboardConfig")
            .field("in_production", &self.in_production)
            .field("admin_password", &set(&self.admin_password))
            .field("github_repo", &self.github_repo)
            .field("github_token", &set(&self.github_token))
            .field("github_api", &self.github_api)
            .field("github_fixture", &self.github_fixture.is_some())
            .field("test_database_url", &set(&self.test_database_url))
            .field("demo_history", &self.demo_history)
            .field("history", &self.history)
            .field("history_path", &self.history_path)
            .field("storage_dir", &self.storage_dir)
            .field("profiler", &self.profiler)
            .field("profiler_at_boot", &self.profiler_at_boot)
            .field("rum", &self.rum)
            .field("rum_sample", &self.rum_sample)
            .field("ai", &self.ai)
            .finish()
    }
}

/// The env vars whose names start with one of `prefixes`, for the dashboard's Config page (which
/// redacts them). This module is the only one that reads the environment.
pub fn shown_env(prefixes: &[&str]) -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(k, _)| prefixes.iter().any(|p| k.starts_with(p)))
        .collect()
}

/// Email (.offrails/guides/mailers.md): which mailer delivers, and as whom.
#[derive(Clone)]
pub struct MailConfig {
    /// `SMTP_URL`: `smtps://user:pass@smtp.example.com:465`. Production sends through it.
    pub smtp_url: Option<String>,
    /// `MAILER`: `smtp`, `local` or `log` overrides the environment's default.
    pub mailer: Option<String>,
    /// `MAIL_FROM`: `Name <address>`.
    pub from: String,
}

impl MailConfig {
    pub fn from_env(_env: AppEnv) -> anyhow::Result<Self> {
        let mailer = std::env::var("MAILER").ok().filter(|s| !s.is_empty());
        if let Some(m) = &mailer
            && !matches!(m.as_str(), "smtp" | "local" | "log")
        {
            anyhow::bail!("MAILER must be smtp, local or log (got {m:?})");
        }
        Ok(Self {
            smtp_url: std::env::var("SMTP_URL").ok().filter(|s| !s.is_empty()),
            mailer,
            from: std::env::var("MAIL_FROM")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("{APP_NAME} <no-reply@localhost>")),
        })
    }

    pub fn test() -> Self {
        Self {
            smtp_url: None,
            mailer: None,
            from: format!("{APP_NAME} <no-reply@localhost>"),
        }
    }
}

// SECURITY: SMTP_URL carries credentials; /dev/introspect.json and logs print Config's Debug (test: the_smtp_url_never_shows_in_config_debug).
impl std::fmt::Debug for MailConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailConfig")
            .field("smtp_url", &self.smtp_url.as_ref().map(|_| "[redacted]"))
            .field("mailer", &self.mailer)
            .field("from", &self.from)
            .finish()
    }
}

/// Accounts (samples/auth-core). `APP_SECRET_KEY` keys the HMAC digests of short secrets
/// (emailed codes, recovery codes, throttle keys), so a database leak alone can't brute-force them.
#[derive(Clone)]
pub struct AuthConfig {
    pub secret_key: Vec<u8>,
    /// Who may create an account: `APP_AUTH_SIGNUP`, else `DEFAULT_SIGNUP`.
    pub signup: Signup,
}

/// Who may create an account. Every sign-in method follows it, so an invite-only app changes
/// one line (or one env var), never its routes or its tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signup {
    /// Anyone: the sign-up form (auth-password) and a magic link's first use (auth-magic-link)
    /// create accounts.
    Open,
    /// By invitation only: `app task auth_invite <email>` (or your admin screen calling
    /// `auth_invite::invite`) creates accounts. The sign-up pages answer 404 and sign-in links go
    /// only to addresses that have an account. Email verification stays: an invitation link
    /// proves the address.
    Invite,
}

/// APP: `Signup::Invite` for an invite-only app (`ACCOUNTS: … invite` in AGENTS.md). Set it here,
/// not only in a deploy's env, so a forgotten variable can't open sign-up.
pub const DEFAULT_SIGNUP: Signup = Signup::Open;

impl Signup {
    /// `APP_AUTH_SIGNUP`: `open` or `invite`; unset or empty means `DEFAULT_SIGNUP`.
    pub fn parse(raw: Option<&str>) -> Result<Self, String> {
        match raw.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") => Ok(DEFAULT_SIGNUP),
            Some("open") => Ok(Signup::Open),
            Some("invite") => Ok(Signup::Invite),
            Some(other) => Err(format!(
                "APP_AUTH_SIGNUP (expected open or invite, got {other:?})"
            )),
        }
    }

    pub fn is_open(self) -> bool {
        self == Signup::Open
    }
}

/// Development only, never accepted in production. Codes made with it are as worthless as the
/// development database they live in.
const DEV_AUTH_SECRET_KEY: &str = "development-only-auth-key-not-a-secret-0001";

impl AuthConfig {
    pub fn from_env(env: AppEnv, missing: &mut Vec<String>, invalid: &mut Vec<String>) -> Self {
        let mut auth =
            Self::from_value(env, std::env::var("APP_SECRET_KEY").ok(), missing, invalid);
        auth.signup = Signup::parse(std::env::var("APP_AUTH_SIGNUP").ok().as_deref())
            .unwrap_or_else(|problem| {
                invalid.push(problem);
                DEFAULT_SIGNUP
            });
        auth
    }

    pub fn from_value(
        env: AppEnv,
        given: Option<String>,
        missing: &mut Vec<String>,
        invalid: &mut Vec<String>,
    ) -> Self {
        match given.filter(|v| !v.trim().is_empty()) {
            // SECURITY: at least 32 characters (`openssl rand -base64 48`); a short key is
            // guessable (test: the_secret_key_must_be_long_enough_and_set_in_production)
            Some(key) if key.len() >= 32 => Self {
                secret_key: key.into_bytes(),
                signup: DEFAULT_SIGNUP,
            },
            Some(_) => {
                invalid.push(
                    "APP_SECRET_KEY (at least 32 characters: openssl rand -base64 48)".into(),
                );
                Self::test()
            }
            // SECURITY: production refuses to boot without its own key
            // (test: the_secret_key_must_be_long_enough_and_set_in_production)
            None if env == AppEnv::Production => {
                missing.push("APP_SECRET_KEY".into());
                Self::test()
            }
            None => Self {
                secret_key: DEV_AUTH_SECRET_KEY.as_bytes().to_vec(),
                signup: DEFAULT_SIGNUP,
            },
        }
    }

    /// A fresh random key per test config: no fixed secret in the test suite.
    pub fn test() -> Self {
        let mut key = vec![0u8; 32];
        rand::fill(&mut key[..]);
        Self {
            secret_key: key,
            signup: DEFAULT_SIGNUP,
        }
    }
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("secret_key", &"[redacted]")
            .field("signup", &self.signup)
            .finish()
    }
}

/// Magic links (samples/auth-magic-link).
#[derive(Clone, Debug, Default)]
pub struct MagicLinkConfig {
    /// `APP_MAGIC_LINK_ANY_BROWSER=true` lets a link work in any browser. Off by default: a link
    /// works only where it was requested (the emailed code covers the other-device case), which
    /// stops login CSRF and forwarded links.
    pub any_browser: bool,
}

impl MagicLinkConfig {
    pub fn from_env(invalid: &mut Vec<String>) -> Self {
        let any_browser = match std::env::var("APP_MAGIC_LINK_ANY_BROWSER").ok().as_deref() {
            None | Some("") | Some("false") | Some("0") => false,
            Some("true") | Some("1") => true,
            Some(_) => {
                invalid.push("APP_MAGIC_LINK_ANY_BROWSER (expected true or false)".into());
                false
            }
        };
        Self { any_browser }
    }
}

/// The job queue's settings (.offrails/guides/jobs.md#queues--configuration).
#[derive(Debug, Clone)]
pub struct JobsConfig {
    /// `APP_QUEUES`: `name:limit[:rate]`, comma-separated. The limit is per node; the rate
    /// (`30/1m`) is global across nodes.
    pub queues: Vec<crate::kernel::jobs::QueueSpec>,
    /// `APP_INLINE_WORKER`: run the worker inside `app serve` (default: development only).
    pub inline_worker: bool,
    /// `APP_JOBS_PRUNE_DAYS`: finished jobs are deleted after this many days (default 7).
    pub prune_days: u32,
    /// `APP_JOBS_SHUTDOWN_GRACE`: seconds in-flight jobs get after SIGTERM (default 25, 3 in
    /// development; keep it below the platform's kill timeout).
    pub shutdown_grace: Duration,
    /// `APP_JOBS_NODE`: this process's name on the dashboard. Default: a random `node-<hex>`,
    /// never the hostname.
    pub node: Option<String>,
}

impl JobsConfig {
    /// APP: the queues every node runs unless `APP_QUEUES` says otherwise. `webhooks` serves the
    /// example workers (src/jobs/examples.rs); drop it with them.
    pub const DEFAULT_QUEUES: &'static str = "default:10,mailers:5,webhooks:3:30/1m";

    pub fn from_env(env: AppEnv) -> anyhow::Result<Self> {
        let get = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let mut invalid: Vec<String> = Vec::new();
        let queues = crate::kernel::jobs::QueueSpec::parse_list(
            get("APP_QUEUES").as_deref().unwrap_or(Self::DEFAULT_QUEUES),
        )
        .unwrap_or_else(|e| {
            invalid.push(format!("APP_QUEUES ({e})"));
            Vec::new()
        });
        let inline_worker = match get("APP_INLINE_WORKER").as_deref() {
            None => env == AppEnv::Development,
            Some("true" | "1") => true,
            Some("false" | "0") => false,
            Some(_) => {
                invalid.push("APP_INLINE_WORKER (expected true or false)".into());
                false
            }
        };
        let mut number =
            |name: &str, default: u64, range: std::ops::RangeInclusive<u64>| match get(name)
                .map(|v| v.parse::<u64>())
            {
                None => default,
                Some(Ok(n)) if range.contains(&n) => n,
                Some(_) => {
                    invalid.push(format!(
                        "{name} (expected {}-{})",
                        range.start(),
                        range.end()
                    ));
                    default
                }
            };
        let prune_days = number("APP_JOBS_PRUNE_DAYS", 7, 1..=3650) as u32;
        // WHY: development restarts on every save (bin/dev); a long grace would stall each one.
        let default_grace = if env == AppEnv::Development { 3 } else { 25 };
        let shutdown_grace =
            Duration::from_secs(number("APP_JOBS_SHUTDOWN_GRACE", default_grace, 0..=3600));
        let node = get("APP_JOBS_NODE");
        if let Some(n) = &node
            && !(n.len() <= 64
                && n.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')))
        {
            invalid.push("APP_JOBS_NODE (1-64 characters of a-z, 0-9, - _ . :)".into());
        }
        if !invalid.is_empty() {
            anyhow::bail!(
                "invalid env vars: {} (see .env.example)",
                invalid.join(", ")
            );
        }
        Ok(Self {
            queues,
            inline_worker,
            prune_days,
            shutdown_grace,
            node,
        })
    }

    pub fn test() -> Self {
        Self {
            queues: crate::kernel::jobs::QueueSpec::parse_list(Self::DEFAULT_QUEUES)
                .unwrap_or_default(),
            inline_worker: false,
            prune_days: 7,
            shutdown_grace: Duration::from_secs(1),
            node: None,
        }
    }
}

/// Access to `/admin/jobs` outside development (controllers/admin_access.rs).
#[derive(Clone, Default)]
pub struct JobsDashboardConfig {
    /// `APP_ADMIN_PASSWORD`: HTTP Basic password for the dashboard when the app has no admin
    /// accounts. Unset in production (and no accounts): the dashboard isn't mounted at all.
    pub admin_password: Option<String>,
}

impl JobsDashboardConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::new(
            std::env::var("APP_ADMIN_PASSWORD")
                .ok()
                .filter(|v| !v.is_empty()),
        )
    }

    pub fn new(admin_password: Option<String>) -> anyhow::Result<Self> {
        // SECURITY: a short password behind HTTP Basic is guessable even with the failed-try
        // limit; refuse to boot with one (test: a_short_admin_password_is_refused).
        if admin_password
            .as_ref()
            .is_some_and(|p| p.chars().count() < 16)
        {
            anyhow::bail!(
                "invalid env vars: APP_ADMIN_PASSWORD (at least 16 characters; see .env.example)"
            );
        }
        Ok(Self { admin_password })
    }
}

impl std::fmt::Debug for JobsDashboardConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobsDashboardConfig")
            .field(
                "admin_password",
                &self.admin_password.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

// samples: config sections

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_config_never_reads_the_environment() {
        let c = Config::test();
        assert_eq!(c.database_url, "sqlite::memory:");
        assert!(!c.is_production() && !c.is_development());
    }

    #[test]
    fn debug_redacts_credentials() {
        let mut c = Config::test();
        c.database_url = "postgres://user:hunter2@db.example.com/app".into();
        assert!(!format!("{c:?}").contains("hunter2"));
    }

    #[test]
    fn a_stale_local_app_url_follows_the_port_being_served() {
        let dev = AppEnv::Development;
        assert_eq!(
            resolve_app_url(dev, Some("http://localhost:3000/"), 3960, false),
            "http://localhost:3960"
        );
        assert_eq!(
            resolve_app_url(dev, Some("http://127.0.0.1:3000"), 3960, true),
            "https://localhost:3960"
        );
        assert_eq!(
            resolve_app_url(dev, None, 3960, false),
            "http://localhost:3960"
        );
        // A tunnel or a LAN name is kept: that's where the links must go.
        assert_eq!(
            resolve_app_url(dev, Some("https://dev.example.com"), 3960, false),
            "https://dev.example.com"
        );
        assert_eq!(
            resolve_app_url(
                AppEnv::Production,
                Some("https://app.example.com/"),
                3960,
                false
            ),
            "https://app.example.com"
        );
    }

    #[test]
    fn development_binds_loopback_unless_told_otherwise() {
        let local = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let any = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        assert_eq!(bind_address(AppEnv::Development, None), Ok(local));
        assert_eq!(bind_address(AppEnv::Development, Some("")), Ok(local));
        // A container opts in.
        assert_eq!(bind_address(AppEnv::Development, Some("0.0.0.0")), Ok(any));
        assert_eq!(
            bind_address(AppEnv::Development, Some("[::1]")),
            Ok("::1".parse().unwrap())
        );
        // Production keeps listening on every interface (its proxy is in front).
        assert_eq!(bind_address(AppEnv::Production, None), Ok(any));
        assert!(bind_address(AppEnv::Development, Some("localhost")).is_err());
        assert_eq!(
            parse_allowed_hosts(Some(" Dev.Example.com, .tunnel.example ,")),
            ["dev.example.com", ".tunnel.example"]
        );
    }

    #[test]
    fn the_development_url_follows_port_and_tls() {
        assert_eq!(dev_app_url(3001, false), "http://localhost:3001");
        assert_eq!(dev_app_url(3000, true), "https://localhost:3000");
    }
}
