use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

pub const AUTH_COOKIE: &str = "auth_token";

// Pulls the logged-in user's id out of the auth cookie, if present and valid.
pub fn current_user_id(jar: &CookieJar, secret: &[u8]) -> Option<Uuid> {
    let token = jar.get(AUTH_COOKIE)?.value();
    crate::jwt::decode_token(token, secret).ok().map(|c| c.sub)
}
