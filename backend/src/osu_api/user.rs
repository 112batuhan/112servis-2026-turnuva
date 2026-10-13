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

// GET /me/osu — the authenticated user's own profile, with osu!standard stats
// whatever their default mode is.
pub async fn fetch_current(
    http: &reqwest::Client,
    access_token: &str,
) -> Result<OsuProfile, AppError> {
    http.get(format!("{}/me/osu", super::API_BASE))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(AppError::OsuProfileRequest)?
        .json()
        .await
        .map_err(AppError::OsuProfileDecode)
}

// The most ids GET /users accepts per request.
pub const USERS_PER_REQUEST: usize = 50;

#[derive(Debug, Deserialize)]
struct UsersResponse {
    users: Vec<OsuUserCompact>,
}

// Trimmed shape of a user from GET /users. Its stats come per ruleset, and it has
// no country rank.
#[derive(Debug, Deserialize)]
pub struct OsuUserCompact {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: Option<String>,
    #[serde(default)]
    statistics_rulesets: StatisticsRulesets,
}

#[derive(Debug, Default, Deserialize)]
struct StatisticsRulesets {
    #[serde(default)]
    osu: OsuStatistics,
}

impl OsuUserCompact {
    pub fn global_rank(&self) -> Option<i64> {
        self.statistics_rulesets.osu.global_rank
    }
}

// GET /users?ids[]= — up to USERS_PER_REQUEST users at once. Missing (restricted
// or deleted) users are left out of the response.
pub async fn fetch_many(
    http: &reqwest::Client,
    access_token: &str,
    ids: &[i64],
) -> Result<Vec<OsuUserCompact>, AppError> {
    let query: Vec<_> = ids.iter().map(|id| ("ids[]", id)).collect();
    let response: UsersResponse = http
        .get(format!("{}/users", super::API_BASE))
        .bearer_auth(access_token)
        .query(&query)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(AppError::OsuApiRequest)?
        .json()
        .await
        .map_err(AppError::OsuApiDecode)?;
    Ok(response.users)
}
