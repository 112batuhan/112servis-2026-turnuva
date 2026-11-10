use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration as StdDuration, Instant},
};

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, ErrorCode},
    role::{Role, Roles},
    AppState,
};

pub const AUTH_COOKIE: &str = "session";

// Carries an OAuth flow's `state` from its start to its callback, so the callback only
// goes through in the browser that started it.
const OAUTH_STATE_COOKIE: &str = "oauth_state";

// How long an OAuth flow may take between its start and its callback.
const OAUTH_STATE_TTL: Duration = Duration::minutes(10);

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
    pub roles: Roles,
    pub discord_verified: bool,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        <AuthUser as OptionalFromRequestParts<AppState>>::from_request_parts(parts, state)
            .await?
            .ok_or(AppError::Unauthenticated)
    }
}

// Signed out (no cookie, or a session that's gone) is `None`; a database failure is
// still an error rather than passing for signed out.
impl OptionalFromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        SessionKey::of(&CookieJar::from_headers(&parts.headers))
            .user(&state.db)
            .await
    }
}

// The session a request came with, kept by a socket that outlives the request so it can
// check again who it belongs to and what they hold.
#[derive(Debug, Clone)]
pub struct SessionKey(Option<Vec<u8>>);

impl SessionKey {
    pub fn of(jar: &CookieJar) -> Self {
        Self(
            jar.get(AUTH_COOKIE)
                .map(|cookie| hash_token(cookie.value())),
        )
    }

    // Its user as of now; None once it's ended or expired.
    pub async fn user(&self, db: &PgPool) -> Result<Option<AuthUser>, AppError> {
        let Some(hash) = &self.0 else {
            return Ok(None);
        };
        let user = db::session::find_user(db, hash).await?;
        Ok(user.map(|user| AuthUser {
            id: user.id,
            roles: Roles::of(&user.roles),
            discord_verified: user.discord_verified,
        }))
    }
}

impl AuthUser {
    // Requires being allowed what the role is (holding it, or being a host), otherwise
    // 403.
    pub fn require_role(&self, required: Role) -> Result<(), AppError> {
        if self.roles.allows(required) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    // Requires a linked Discord account, otherwise 403. Fails closed — an
    // unverified caller is rejected.
    pub fn require_discord_verified(&self) -> Result<(), AppError> {
        if self.discord_verified {
            Ok(())
        } else {
            Err(AppError::DiscordVerificationRequired)
        }
    }

    // The team the caller plays in, otherwise 400.
    pub async fn team_id(&self, db: &PgPool) -> Result<Uuid, AppError> {
        db::team::find_team_id(db, self.id)
            .await?
            .ok_or(AppError::BadRequest(ErrorCode::NotInTeam))
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
    crate::supervise::spawn("session cleanup", move || clean_sessions(pool.clone()));
}

async fn clean_sessions(pool: PgPool) {
    {
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
    }
}

// Which OAuth flow a `state` was handed out for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthFlow {
    OsuLogin,
    DiscordLink,
}

#[derive(Debug, Clone, Copy)]
struct PendingFlow {
    flow: OAuthFlow,
    // Who started it (the Discord link flow needs a session).
    user_id: Option<Uuid>,
    started: Instant,
}

// The OAuth flows started in the last OAUTH_STATE_TTL, by their `state`. The state is
// also set in the starting browser's cookie, and a callback has to bring both.
#[derive(Clone, Default)]
pub struct OAuthStates(Arc<Mutex<HashMap<String, PendingFlow>>>);

fn state_expired(started: Instant) -> bool {
    started.elapsed() > StdDuration::from_secs(OAUTH_STATE_TTL.whole_seconds() as u64)
}

impl OAuthStates {
    // Records a new flow and returns the cookie that carries its state, dropping the
    // flows that ran out.
    pub fn start(
        &self,
        state: &str,
        flow: OAuthFlow,
        user_id: Option<Uuid>,
        secure: bool,
    ) -> Cookie<'static> {
        let mut states = self.0.lock().unwrap();
        states.retain(|_, pending| !state_expired(pending.started));
        states.insert(
            state.to_string(),
            PendingFlow {
                flow,
                user_id,
                started: Instant::now(),
            },
        );
        let mut cookie = Cookie::new(OAUTH_STATE_COOKIE, state.to_string());
        cookie.set_http_only(true);
        cookie.set_path("/auth");
        cookie.set_same_site(SameSite::Lax);
        cookie.set_max_age(OAUTH_STATE_TTL);
        cookie.set_secure(secure);
        cookie
    }

    // Ends the flow a callback names. It only counts when the browser's cookie holds the
    // same state, the flow is the one expected and it hasn't run out; returns who
    // started it, and the jar with the cookie cleared either way.
    pub fn finish(
        &self,
        jar: CookieJar,
        state: Option<&str>,
        flow: OAuthFlow,
    ) -> (CookieJar, Option<Option<Uuid>>) {
        let cookie = jar.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_string());
        let jar = jar.remove(Cookie::build(OAUTH_STATE_COOKIE).path("/auth"));
        let Some(state) = state else {
            return (jar, None);
        };
        let pending = self.0.lock().unwrap().remove(state);
        let valid = cookie.as_deref() == Some(state)
            && pending.is_some_and(|p| p.flow == flow && !state_expired(p.started));
        (jar, valid.then(|| pending.and_then(|p| p.user_id)))
    }
}
