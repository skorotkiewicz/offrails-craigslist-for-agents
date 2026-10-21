//! The framework runtime, generated from the guide specs (.offrails/stack.md#the-kernel-code-you-generate-instead-of-depending-on).

pub mod assets;
pub mod auth_token;
pub mod background;
pub mod cache;
pub mod cookie;
pub mod css;
pub mod dashboard;
pub mod db;
pub mod flash;
pub mod hosts;
pub mod introspect;
pub mod jobs;
pub mod jobs_web;
pub mod mailbox;
pub mod mailbox_cli;
pub mod mailbox_web;
pub mod mailer;
pub mod outbound;
pub mod rate_limit;
pub mod redact;
pub mod return_to;
pub mod security;
pub mod session;
pub mod theme;
pub mod tls;
pub mod ui;
pub mod user_agent;
// samples: kernel mods
