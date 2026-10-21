//! `lib/tasks`: `app task <name> [args…]`.

pub mod auth_cleanup;
pub mod auth_demo;
pub mod auth_invite;
pub mod jobs_demo;
pub mod seed;
pub mod send_test_email;
pub mod test_db_cleanup;
pub mod tokens; // api
// samples: task mods

use crate::state::AppState;

pub async fn run(state: &AppState, name: &str, args: Vec<String>) -> anyhow::Result<()> {
    match name {
        "seed" => seed::run(state, args).await,
        "test_db_cleanup" => test_db_cleanup::run(state, args).await,
        "send_test_email" => send_test_email::run(state, args).await,
        "auth_cleanup" => auth_cleanup::run(state, args).await,
        "auth_demo" => auth_demo::run(state, args).await,
        "auth_invite" => auth_invite::run(state, args).await,
        "jobs_demo" => jobs_demo::run(state, args).await,
        // api
        "tokens" => tokens::run(state, args).await,
        // samples: tasks
        other => anyhow::bail!("unknown task: {other}"),
    }
}
