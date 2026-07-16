use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sqlx::PgPool;
use time::Duration;
use uuid::Uuid;

use crate::{error::AppError, jwt, role::Role};

pub const AUTH_COOKIE: &str = "auth_token";

// The authenticated caller, as carried by their JWT (no database lookup).
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
    pub token_version: i32,
    pub discord_verified: bool,
}

impl AuthUser {
    // Guard for endpoints that require a linked Discord account. Fails closed —
    // an unverified caller is rejected. Wire this into forthcoming handlers.
    #[allow(dead_code)]
    pub fn require_discord_verified(&self) -> Result<(), AppError> {
        if self.discord_verified {
            Ok(())
        } else {
            Err(AppError::DiscordVerificationRequired)
        }
    }
}

// Decodes the auth cookie into an AuthUser using only the JWT — fast, no DB hit.
// The role may be up to one token lifetime stale after a role change; fine for
// low-stakes gating. Use `require_role` for anything privileged.
pub fn current_user(jar: &CookieJar, secret: &[u8]) -> Option<AuthUser> {
    let token = jar.get(AUTH_COOKIE)?.value();
    let claims = jwt::decode_token(token, secret).ok()?;
    Some(AuthUser {
        id: claims.sub,
        role: claims.role,
        token_version: claims.tv,
        discord_verified: claims.discord_verified,
    })
}

// Convenience for endpoints that only need "who is this", not the role.
pub fn current_user_id(jar: &CookieJar, secret: &[u8]) -> Option<Uuid> {
    current_user(jar, secret).map(|u| u.id)
}

// Authenticates AND confirms the token hasn't been superseded by a role change,
// via a single indexed lookup of the stored token_version. Use this wherever a
// stale role would be a problem — i.e. privileged mutations.
pub async fn verify_user(
    pool: &PgPool,
    jar: &CookieJar,
    secret: &[u8],
) -> Result<AuthUser, AppError> {
    let user = current_user(jar, secret).ok_or(AppError::Unauthenticated)?;
    let stored_version: i32 =
        sqlx::query_scalar("SELECT token_version FROM osu_users WHERE id = $1")
            .bind(user.id)
            .fetch_optional(pool)
            .await?
            .ok_or(AppError::Unauthenticated)?;
    if stored_version != user.token_version {
        return Err(AppError::Unauthenticated);
    }
    Ok(user)
}

// verify_user + a minimum-privilege check. Returns 403 if the role is too low.
pub async fn require_role(
    pool: &PgPool,
    jar: &CookieJar,
    secret: &[u8],
    required: Role,
) -> Result<AuthUser, AppError> {
    let user = verify_user(pool, jar, secret).await?;
    if !user.role.has_at_least(required) {
        return Err(AppError::Forbidden);
    }
    Ok(user)
}

// Builds the httpOnly session cookie carrying a signed JWT. Shared by login and
// the /api/me refresh so the cookie attributes stay in one place.
pub fn auth_cookie(token: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(AUTH_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Duration::seconds(jwt::TOKEN_TTL_SECS as i64));
    // TODO: set_secure(true) once served over HTTPS.
    cookie.set_secure(false);
    cookie
}
