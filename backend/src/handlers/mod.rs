//! HTTP request handlers, grouped by feature area.
//!
//! Handlers return `Result<impl IntoResponse, crate::error::AppError>` and pull
//! shared services (db pool, http client, oauth clients, config) off `AppState`.
pub mod admin;
pub mod auth;
pub mod bracket;
pub mod dev;
pub mod irc;
pub mod mappool;
pub mod matches;
pub mod notifications;
pub mod referee;
pub mod reschedule;
pub mod team;
pub mod user;
