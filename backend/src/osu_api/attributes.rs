use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;

// osu!standard difficulty attributes (POST /beatmaps/{id}/attributes). Only the star
// rating is read: the response no longer carries the mod-adjusted AR and OD, so those
// are computed in `mods.rs` like CS, BPM and length.
#[derive(Debug, Deserialize)]
pub struct DifficultyAttributes {
    pub star_rating: f64,
}

#[derive(Debug, Deserialize)]
struct AttributesResponse {
    attributes: DifficultyAttributes,
}

// Requests the difficulty attributes for a beatmap with the given mod acronyms
// applied (e.g. ["HD", "DT"]). Uses the beatmap's own ruleset.
pub async fn fetch(
    http: &reqwest::Client,
    access_token: &str,
    beatmap_id: i64,
    mods: &[String],
) -> Result<DifficultyAttributes, AppError> {
    let resp = http
        .post(format!(
            "{}/beatmaps/{beatmap_id}/attributes",
            super::API_BASE
        ))
        .bearer_auth(access_token)
        .json(&json!({ "mods": mods }))
        .send()
        .await
        .map_err(AppError::OsuApiRequest)?
        .error_for_status()
        .map_err(AppError::OsuApiRequest)?;

    let parsed: AttributesResponse = resp.json().await.map_err(AppError::OsuApiDecode)?;
    Ok(parsed.attributes)
}
