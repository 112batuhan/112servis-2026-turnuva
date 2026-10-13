use std::time::Duration as StdDuration;

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{db, error::AppError, role::Role, AppState};

pub const AUTH_COOKIE: &str = "session";

// Sessions last 7 days from login; there's no sliding renewal.
const SESSION_TTL: Duration = Duration::days(7);

// How often the background task sweeps expired sessions out of the table.
const CLEANUP_INTERVAL: StdDuration = StdDuration::from_secs(60 * 60);

// The authenticated caller, resolved from the session cookie. Used as an extractor:
// a handler that takes `AuthUser` runs only for a live session, and one that takes
// `Option<AuthUser>` sees `None` when signed out. Every extraction reads the session
// and user from Postgres, so role changes and logouts take effect on the next request.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
    pub discord_verified: bool,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let cookie = jar.get(AUTH_COOKIE).ok_or(AppError::Unauthenticated)?;
        let user = db::session::find_user(&state.db, &hash_token(cookie.value()))
            .await?
            .ok_or(AppError::Unauthenticated)?;
        Ok(AuthUser {
            id: user.id,
            role: user.role,
            discord_verified: user.discord_verified,
        })
    }
}

impl AuthUser {
    // Requires at least the given role, otherwise 403.
    pub fn require_role(&self, required: Role) -> Result<(), AppError> {
        if self.role.has_at_least(required) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    // Requires a linked Discord account, otherwise 403. Fails closed — an
    // unverified caller is rejected. Wire this into forthcoming handlers.
    #[allow(dead_code)]
    pub fn require_discord_verified(&self) -> Result<(), AppError> {
        if self.discord_verified {
            Ok(())
        } else {
            Err(AppError::DiscordVerificationRequired)
        }
    }
}

// Only the hash of a session token is stored, so the raw cookie value can't be
// recovered from the database.
fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

// Starts a session for `user_id` and returns the cookie that carries it.
pub async fn start_session(state: &AppState, user_id: Uuid) -> Result<Cookie<'static>, AppError> {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

    let expires_at = OffsetDateTime::now_utc() + SESSION_TTL;
    db::session::create(&state.db, &hash_token(&token), user_id, expires_at).await?;
    Ok(auth_cookie(token, state.config.secure_cookies()))
}

// Ends the session named by the request's cookie, if any, and returns the jar with
// the cookie cleared.
pub async fn end_session(state: &AppState, jar: CookieJar) -> Result<CookieJar, AppError> {
    if let Some(cookie) = jar.get(AUTH_COOKIE) {
        db::session::delete(&state.db, &hash_token(cookie.value())).await?;
    }
    // The removal cookie must match the original's path for the browser to drop it.
    Ok(jar.remove(Cookie::build(AUTH_COOKIE).path("/")))
}

// Builds the httpOnly session cookie. `secure` should be true whenever the app is
// served over HTTPS (see Config::secure_cookies, derived from FRONTEND_URL's
// scheme) — browsers silently drop `Secure` cookies set over plain HTTP, so this
// can't just always be true, and leaving it false in production would let the
// session cookie travel unencrypted.
fn auth_cookie(token: String, secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::new(AUTH_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(SESSION_TTL);
    cookie.set_secure(secure);
    cookie
}

// Runs alongside the server for its whole lifetime, deleting expired sessions every
// CLEANUP_INTERVAL (the first sweep happens at startup). Failures are logged and
// retried on the next tick rather than taking the server down.
pub fn spawn_session_cleanup(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            match db::session::delete_expired(&pool).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("removed {n} expired sessions"),
                Err(err) => tracing::error!(error = ?err, "session cleanup failed"),
            }
        }
    });
}
