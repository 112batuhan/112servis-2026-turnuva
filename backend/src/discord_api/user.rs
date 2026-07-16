use serde::Deserialize;

use crate::error::AppError;

// Trimmed shape of Discord's GET /users/@me response.
#[derive(Debug, Deserialize)]
pub struct DiscordProfile {
    pub id: String,
    pub username: String,
    pub avatar: Option<String>,
}

// GET /users/@me — the authenticated user's own Discord profile.
pub async fn fetch_current(
    http: &reqwest::Client,
    access_token: &str,
) -> Result<DiscordProfile, AppError> {
    http.get(format!("{}/users/@me", super::API_BASE))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(AppError::DiscordProfileRequest)?
        .json()
        .await
        .map_err(AppError::DiscordProfileDecode)
}
