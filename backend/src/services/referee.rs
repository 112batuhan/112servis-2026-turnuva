//! What both a match's referee (handlers/referee.rs) and the auto referee
//! (bancho/manager/auto_referee.rs) do to the match's room settings, so the pages hear
//! of it the same way whoever did it.

use std::collections::HashMap;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{db, error::AppError, osu_api::matches::MatchGame, AppState};

// osu!'s record of the match's rooms (read with `bancho::read_osu_games`) takes the place
// of the chat's maps up to its last one, for every page, and the bots save the score
// counted from it. Returns when the match was synced.
pub async fn apply_osu_sync(
    state: &AppState,
    match_id: Uuid,
    games: HashMap<i64, Vec<MatchGame>>,
) -> Result<OffsetDateTime, AppError> {
    let at = db::referee::mark_osu_synced(&state.db, match_id).await?;
    state.bancho.osu_games(match_id, at, games);
    state.live.referee_changed(match_id);
    Ok(at)
}

// Whose ban or pick is next in a bracket match (None: as the stage's order says), until
// the next one is marked: the referee's override, or the roll's winner. The match page
// shows it.
pub async fn set_next_pick(
    state: &AppState,
    match_id: Uuid,
    team_id: Option<Uuid>,
) -> Result<(), AppError> {
    db::referee::set_next_pick(&state.db, match_id, team_id).await?;
    state.live.match_changed(match_id);
    Ok(())
}
