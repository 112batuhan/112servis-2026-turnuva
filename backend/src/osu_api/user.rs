use serde::Deserialize;

use crate::error::AppError;

// Trimmed shape of osu!'s GET /api/v2/me response.
#[derive(Debug, Deserialize)]
pub struct OsuProfile {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: Option<String>,
    #[serde(default)]
    pub statistics: OsuStatistics,
}

#[derive(Debug, Default, Deserialize)]
pub struct OsuStatistics {
    pub global_rank: Option<i64>,
    pub country_rank: Option<i64>,
}

// GET /me — the authenticated user's own profile.
pub async fn fetch_current(http: &reqwest::Client, access_token: &str) -> Result<OsuProfile, AppError> {
    http.get(format!("{}/me", super::API_BASE))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(AppError::OsuProfileRequest)?
        .json()
        .await
        .map_err(AppError::OsuProfileDecode)
}
