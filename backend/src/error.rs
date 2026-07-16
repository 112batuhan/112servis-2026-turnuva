use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

// Central error type for handlers: `?`-friendly, and turns into an HTTP response
// that hides internal detail from the client while still logging it server-side.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("not authenticated")]
    Unauthenticated,

    #[error("insufficient permissions")]
    Forbidden,

    #[error("discord verification required")]
    DiscordVerificationRequired,

    #[error("invalid or expired login attempt")]
    InvalidLoginAttempt,

    #[error("osu! token exchange failed: {0}")]
    OsuTokenExchange(String),

    #[error("could not reach osu!")]
    OsuProfileRequest(#[source] reqwest::Error),

    #[error("could not read osu! profile")]
    OsuProfileDecode(#[source] reqwest::Error),

    #[error("discord token exchange failed: {0}")]
    DiscordTokenExchange(String),

    #[error("could not reach Discord")]
    DiscordProfileRequest(#[source] reqwest::Error),

    #[error("could not read Discord profile")]
    DiscordProfileDecode(#[source] reqwest::Error),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("failed to sign session token")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message): (StatusCode, &'static str) = match &self {
            AppError::Unauthenticated => (StatusCode::UNAUTHORIZED, "not authenticated"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "insufficient permissions"),
            AppError::DiscordVerificationRequired => {
                (StatusCode::FORBIDDEN, "discord verification required")
            }
            AppError::InvalidLoginAttempt => {
                (StatusCode::BAD_REQUEST, "invalid or expired login attempt")
            }
            AppError::OsuTokenExchange(_)
            | AppError::OsuProfileRequest(_)
            | AppError::OsuProfileDecode(_) => {
                (StatusCode::BAD_GATEWAY, "could not complete osu! login")
            }
            AppError::DiscordTokenExchange(_)
            | AppError::DiscordProfileRequest(_)
            | AppError::DiscordProfileDecode(_) => {
                (StatusCode::BAD_GATEWAY, "could not link Discord")
            }
            AppError::Database(_) | AppError::Jwt(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };

        // Client only sees the generic message above; the real cause goes to the logs.
        if status.is_server_error() {
            tracing::error!(error = ?self, "{self}");
        }

        (status, message).into_response()
    }
}
