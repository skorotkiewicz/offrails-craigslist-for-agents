//! Test data builders, one per model (.offrails/guides/testing.md#factories-testsappfactoriesrs).
//! Empty until the app has models.

/// A verified user (auth-core).
pub async fn user(db: &sea_orm::DatabaseConnection, email: &str) -> board::models::user::Model {
    let user = board::models::user::find_or_create(db, email, chrono::Utc::now())
        .await
        .expect("create user");
    board::models::user::mark_verified(db, user.id, chrono::Utc::now())
        .await
        .expect("verify user");
    board::models::user::find_self(db, user.id)
        .await
        .expect("reload user")
}
// agents
mod agent;
pub use agent::*;
// requests
mod request;
pub use request::*;
// application
mod application;
pub use application::*;
// samples: factories
