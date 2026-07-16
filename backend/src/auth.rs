use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;
use uuid::Uuid;

use crate::{error::AppError, jwt, role::Role, AppState};

pub const AUTH_COOKIE: &str = "auth_token";

// The authenticated caller, decoded from the JWT cookie. Used as an extractor:
// a handler that takes `AuthUser` runs only for a valid token, and one that takes
// `Option<AuthUser>` sees `None` when signed out. Extraction is pure JWT work —
// it never touches the database, so it carries the token's snapshot of the user
// (role, discord_verified) which callers authorize against directly. Staleness
// after a role change is reconciled by /api/me, not on every request.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
    pub token_version: i32,
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
        let claims = jwt::decode_token(cookie.value(), state.config.jwt_secret.as_bytes())
            .map_err(|_| AppError::Unauthenticated)?;
        Ok(AuthUser {
            id: claims.sub,
            role: claims.role,
            token_version: claims.tv,
            discord_verified: claims.discord_verified,
        })
    }
}

impl AuthUser {
    // Requires at least the given role, otherwise 403. Pure JWT check, no DB.
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

// Builds the httpOnly session cookie carrying a signed JWT. Shared by login and
// the /api/me refresh so the cookie attributes stay in one place. `secure` should
// be true whenever the app is served over HTTPS (see AppState::secure_cookies,
// derived from FRONTEND_URL's scheme) — browsers silently drop `Secure` cookies
// set over plain HTTP, so this can't just always be true, and leaving it false in
// production would let the session cookie travel unencrypted.
pub fn auth_cookie(token: String, secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::new(AUTH_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Duration::seconds(jwt::TOKEN_TTL_SECS as i64));
    cookie.set_secure(secure);
    cookie
}
