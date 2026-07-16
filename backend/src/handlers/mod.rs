//! HTTP request handlers, grouped by feature area.
//!
//! Handlers return `Result<impl IntoResponse, crate::error::AppError>` and pull
//! shared services (db pool, http client, oauth clients, config) off `AppState`.
pub mod auth;
pub mod user;
