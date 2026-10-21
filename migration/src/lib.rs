pub use sea_orm_migration::MigrationStatus;
pub use sea_orm_migration::prelude::*;

mod m20261016_000000_create_cache_entries;
mod m20261016_000100_create_mailbox_messages;
mod m20261016_000200_create_jobs;
mod m20261016_000600_create_lab_orders;
mod m20261016_001000_create_users;
mod m20261016_001100_create_sessions;
mod m20261016_001200_create_user_tokens;
mod m20261016_001300_create_auth_events_and_throttles;
mod m20261016_003000_create_login_links;
mod m20261021_191809_create_agents; // agents
mod m20261021_191811_create_requests; // requests
mod m20261021_191825_create_applications; // application
mod m20261021_191827_create_api_tokens; // api
// samples: migration mods

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261016_000000_create_cache_entries::Migration),
            Box::new(m20261016_000600_create_lab_orders::Migration),
            Box::new(m20261016_000100_create_mailbox_messages::Migration),
            Box::new(m20261016_001000_create_users::Migration),
            Box::new(m20261016_001100_create_sessions::Migration),
            Box::new(m20261016_001200_create_user_tokens::Migration),
            Box::new(m20261016_001300_create_auth_events_and_throttles::Migration),
            Box::new(m20261016_003000_create_login_links::Migration),
            Box::new(m20261016_000200_create_jobs::Migration),
            // agents
            Box::new(m20261021_191809_create_agents::Migration),
            // requests
            Box::new(m20261021_191811_create_requests::Migration),
            // application
            Box::new(m20261021_191825_create_applications::Migration),
            // api
            Box::new(m20261021_191827_create_api_tokens::Migration),
            // samples: migrations
        ]
    }
}
