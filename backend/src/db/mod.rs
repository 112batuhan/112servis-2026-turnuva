//! Database access layer: the connection pool plus query modules grouped by category.
//!
//! Add a submodule per category (e.g. `pub mod user;`) as new tables appear; query
//! functions take `&PgPool` and return typed rows, surfacing failures as `sqlx::Error`.
pub mod admin;
pub mod bancho;
pub mod bracket;
pub mod chat;
pub mod dev;
pub mod discord;
pub mod mappool;
pub mod matches;
pub mod notifications;
pub mod ranks;
pub mod referee;
pub mod reschedule;
pub mod rules;
pub mod session;
pub mod settings;
pub mod team;
pub mod user;

use sqlx::{postgres::PgPoolOptions, PgPool};

// Connects to postgres and runs pending migrations.
pub async fn connect(database_url: &str) -> PgPool {
    // The Discord job worker's listener keeps one of these for good.
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}
