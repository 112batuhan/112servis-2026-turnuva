use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

mod code;
pub use code::ErrorCode;

// Central error type for handlers: `?`-friendly, and turns into an HTTP response
// that hides internal detail from the client while still logging it server-side.
//
// Responses are JSON, `{ "code": "lobby_full", "message": "that lobby is full" }`: the
// code (an ErrorCode, code.rs) is what the frontend goes by (it shows each in Turkish,
// `api/errors.ts`), the English message is for logs and anything without a translation.
// BadRequest, Conflict and TooManyRequests take the code; the status comes from which.
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

    #[error("not found")]
    NotFound,

    #[error("{}", .0.message())]
    BadRequest(ErrorCode),

    #[error("{}", .0.message())]
    Conflict(ErrorCode),

    #[error("{}", .0.message())]
    TooManyRequests(ErrorCode),

    #[error("beatmap {0} not found")]
    BeatmapNotFound(i64),

    #[error("could not reach osu! API")]
    OsuApiRequest(#[source] reqwest::Error),

    #[error("could not read osu! API response")]
    OsuApiDecode(#[source] reqwest::Error),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("could not serialize a response")]
    Serialize(#[from] serde_json::Error),
}

#[derive(serde::Serialize)]
struct ErrorBody {
    code: ErrorCode,
    message: &'static str,
}

impl AppError {
    // The status and the code the client gets.
    fn public(&self) -> (StatusCode, ErrorCode) {
        match self {
            AppError::Unauthenticated => (StatusCode::UNAUTHORIZED, ErrorCode::Unauthenticated),
            AppError::Forbidden => (StatusCode::FORBIDDEN, ErrorCode::Forbidden),
            AppError::DiscordVerificationRequired => {
                (StatusCode::FORBIDDEN, ErrorCode::DiscordRequired)
            }
            AppError::InvalidLoginAttempt => (StatusCode::BAD_REQUEST, ErrorCode::InvalidLogin),
            AppError::OsuTokenExchange(_)
            | AppError::OsuProfileRequest(_)
            | AppError::OsuProfileDecode(_) => (StatusCode::BAD_GATEWAY, ErrorCode::OsuLoginFailed),
            AppError::DiscordTokenExchange(_)
            | AppError::DiscordProfileRequest(_)
            | AppError::DiscordProfileDecode(_) => {
                (StatusCode::BAD_GATEWAY, ErrorCode::DiscordLinkFailed)
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, ErrorCode::NotFound),
            AppError::BadRequest(code) => (StatusCode::BAD_REQUEST, *code),
            AppError::Conflict(code) => (StatusCode::CONFLICT, *code),
            AppError::TooManyRequests(code) => (StatusCode::TOO_MANY_REQUESTS, *code),
            AppError::BeatmapNotFound(_) => (StatusCode::NOT_FOUND, ErrorCode::BeatmapNotFound),
            AppError::OsuApiRequest(_) => (StatusCode::BAD_GATEWAY, ErrorCode::OsuApiUnreachable),
            AppError::OsuApiDecode(_) => (StatusCode::BAD_GATEWAY, ErrorCode::OsuApiBadResponse),
            AppError::Database(_) | AppError::Serialize(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal)
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.public();

        // Client only sees the code and its generic message; the real cause goes to the
        // logs.
        if status.is_server_error() {
            tracing::error!(error = ?self, "{self}");
        }

        let body = ErrorBody {
            code,
            message: code.message(),
        };
        (status, Json(body)).into_response()
    }
}

// Formats an error together with its full source chain. Useful for otherwise
// opaque transport failures like oauth2's "Request failed", whose real cause
// (connection reset, DNS, TLS, ...) is only in the source.
pub fn chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut source = err.source();
    while let Some(e) = source {
        out.push_str(": ");
        out.push_str(&e.to_string());
        source = e.source();
    }
    out
}
