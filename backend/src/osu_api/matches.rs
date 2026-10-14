use serde::Deserialize;

use crate::error::AppError;

// The most events GET /matches/{id} returns per call.
const EVENTS_PER_REQUEST: usize = 100;

#[derive(Debug, Deserialize)]
struct MatchResponse {
    events: Vec<MatchEvent>,
    latest_event_id: i64,
}

#[derive(Debug, Deserialize)]
struct MatchEvent {
    id: i64,
    #[serde(default)]
    game: Option<MatchGame>,
}

// A map played in a multiplayer room.
#[derive(Debug, Deserialize)]
pub struct MatchGame {
    pub beatmap_id: Option<i64>,
    // Null while the map is still being played.
    pub end_time: Option<String>,
    #[serde(default)]
    pub scores: Vec<GameScore>,
}

#[derive(Debug, Deserialize)]
pub struct GameScore {
    pub user_id: i64,
    #[serde(default)]
    pub score: i64,
}

// GET /matches/{id} — every finished game of a multiplayer room, in the order played.
// Without a cursor the API returns the latest events, so this pages forward from the
// start with `after`.
pub async fn fetch_games(
    http: &reqwest::Client,
    access_token: &str,
    mp_id: i64,
) -> Result<Vec<MatchGame>, AppError> {
    let mut games = Vec::new();
    let mut after = 0;
    loop {
        let page: MatchResponse = http
            .get(format!("{}/matches/{mp_id}", super::API_BASE))
            .bearer_auth(access_token)
            .query(&[("limit", EVENTS_PER_REQUEST as i64), ("after", after)])
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(AppError::OsuApiRequest)?
            .json()
            .await
            .map_err(AppError::OsuApiDecode)?;
        let last = page.events.last().map(|e| e.id);
        games.extend(
            page.events
                .into_iter()
                .filter_map(|e| e.game)
                .filter(|g| g.end_time.is_some()),
        );
        match last {
            Some(last) if last > after && last < page.latest_event_id => after = last,
            _ => break,
        }
    }
    Ok(games)
}
