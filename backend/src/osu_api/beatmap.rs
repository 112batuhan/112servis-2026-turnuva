use serde::Deserialize;

use crate::error::AppError;

// Relevant fields of osu! API v2 GET /beatmaps/{id}. Note the API names OD
// `accuracy` and HP `drain`.
#[derive(Debug, Deserialize)]
pub struct Beatmap {
    pub id: i64,
    pub beatmapset_id: i64,
    pub difficulty_rating: f64,
    #[serde(default)]
    pub bpm: Option<f64>,
    pub total_length: i32,
    pub cs: f64,
    pub ar: f64,
    pub accuracy: f64,
    pub drain: f64,
    pub version: String,
    pub mode: String,
    pub beatmapset: Option<Beatmapset>,
}

#[derive(Debug, Deserialize)]
pub struct Beatmapset {
    pub artist: String,
    pub title: String,
    pub creator: String,
    pub covers: Covers,
}

#[derive(Debug, Deserialize)]
pub struct Covers {
    #[serde(default)]
    pub cover: Option<String>,
}

// GET /beatmaps/{id} — a single beatmap's metadata and difficulty stats.
pub async fn fetch(
    http: &reqwest::Client,
    access_token: &str,
    beatmap_id: i64,
) -> Result<Beatmap, AppError> {
    let resp = http
        .get(format!("{}/beatmaps/{beatmap_id}", super::API_BASE))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(AppError::OsuApiRequest)?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::BeatmapNotFound(beatmap_id));
    }

    resp.error_for_status()
        .map_err(AppError::OsuApiRequest)?
        .json::<Beatmap>()
        .await
        .map_err(AppError::OsuApiDecode)
}
