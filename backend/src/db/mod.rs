//! Database access layer: the connection pool plus query modules grouped by category.
//!
//! Add a submodule per category (e.g. `pub mod user;`) as new tables appear; query
//! functions take `&PgPool` and return typed rows, surfacing failures as `sqlx::Error`.
pub mod admin;
pub mod bracket;
pub mod chat;
pub mod dev;
pub mod mappool;
pub mod matches;
pub mod notifications;
pub mod ranks;
pub mod reschedule;
pub mod session;
pub mod settings;
pub mod team;
pub mod user;

use sqlx::{postgres::PgPoolOptions, PgPool};

// Connects to postgres and runs pending migrations.
pub async fn connect(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}
