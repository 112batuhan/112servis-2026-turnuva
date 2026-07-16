use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use oauth2::{basic::BasicClient, reqwest::async_http_client, Scope, TokenResponse};

use crate::error::AppError;

// A cached osu! app access token obtained via the client-credentials grant, used
// for public API calls (fetching beatmap data). It's an ephemeral credential, so
// it lives in memory — all domain data is persisted in Postgres.
#[derive(Clone)]
pub struct OsuAppToken {
    access_token: String,
    expires_at: Instant,
}

// Returns a valid app token, reusing the cached one until shortly before it
// expires and otherwise fetching a fresh one. Concurrent callers may occasionally
// each fetch once; that's harmless.
pub async fn app_token(
    client: &BasicClient,
    cache: &Mutex<Option<OsuAppToken>>,
) -> Result<String, AppError> {
    {
        let guard = cache.lock().unwrap();
        if let Some(token) = guard.as_ref() {
            if token.expires_at > Instant::now() {
                return Ok(token.access_token.clone());
            }
        }
    }

    let response = client
        .exchange_client_credentials()
        .add_scope(Scope::new("public".to_string()))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::OsuTokenExchange(crate::error::chain(&err)))?;

    let access_token = response.access_token().secret().clone();
    // Refresh a minute early so a token never expires mid-request.
    let ttl = response.expires_in().unwrap_or(Duration::from_secs(3600));
    let expires_at = Instant::now() + ttl.saturating_sub(Duration::from_secs(60));

    *cache.lock().unwrap() = Some(OsuAppToken {
        access_token: access_token.clone(),
        expires_at,
    });
    Ok(access_token)
}
