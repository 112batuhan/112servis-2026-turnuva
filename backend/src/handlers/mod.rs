//! HTTP request handlers, grouped by feature area.
//!
//! Handlers return `Result<impl IntoResponse, crate::error::AppError>` and pull
//! shared services (db pool, http client, oauth clients, config) off `AppState`.
pub mod admin;
pub mod arg;
pub mod auth;
pub mod bancho;
pub mod bracket;
pub mod dev;
pub mod discord;
pub mod irc;
pub mod live;
pub mod mappool;
pub mod matches;
pub mod notifications;
pub mod pickem;
pub mod referee;
pub mod reschedule;
pub mod rules;
pub mod stats;
pub mod team;
pub mod user;
