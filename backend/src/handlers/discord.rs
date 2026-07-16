use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use oauth2::{reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::Deserialize;

use crate::{auth::current_user_id, db, discord_api, error::AppError, AppState};

// GET /auth/discord/link — starts the optional Discord link flow; requires an osu! session already.
pub async fn link(State(state): State<AppState>, jar: CookieJar) -> Result<impl IntoResponse, AppError> {
    if current_user_id(&jar, state.jwt_secret.as_bytes()).is_none() {
        return Err(AppError::Unauthenticated);
    }

    let (auth_url, csrf_token) = state
        .discord_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    state.csrf_tokens.lock().unwrap().insert(csrf_token.secret().clone(), ());
    Ok(Redirect::to(auth_url.as_str()))
}

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

// GET /auth/discord/callback — links the Discord profile to the currently logged-in user.
pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let Some(user_id) = current_user_id(&jar, state.jwt_secret.as_bytes()) else {
        return Err(AppError::Unauthenticated);
    };

    if state.csrf_tokens.lock().unwrap().remove(&params.state).is_none() {
        return Err(AppError::InvalidLoginAttempt);
    }

    let token = state
        .discord_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|err| AppError::DiscordTokenExchange(err.to_string()))?;

    let profile = discord_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    db::link_discord(&state.db, user_id, &profile).await?;

    Ok(Redirect::to(&state.frontend_url))
}
