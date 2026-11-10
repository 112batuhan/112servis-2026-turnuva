//! What's done to any match, whoever asks: the routes in handlers/matches.rs and the
//! room bots (crate::bancho), which put the rooms they make on their matches.

use uuid::Uuid;

use crate::{
    db::{
        self,
        matches::{MatchKind, MatchState},
    },
    discord::jobs::Job,
    error::{AppError, GeneralError, RoomError},
    AppState,
};

// A match that can be run: a potential match has no room, notes or result of its own,
// so the routes that run a match don't know it.
pub async fn load_match(state: &AppState, id: Uuid) -> Result<MatchState, AppError> {
    db::matches::find_match(&state.db, id)
        .await?
        .filter(|m| m.kind != MatchKind::Potential)
        .ok_or(GeneralError::NotFound.into())
}

// Puts a room on its match, from a link or as a bot made it, and tells Discord and the
// pages. `bot` runs it; `bot_made` when that bot made it.
pub async fn link_room(
    state: &AppState,
    match_id: Uuid,
    mp_id: i64,
    bot: Option<Uuid>,
    bot_made: bool,
    concluded: bool,
) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    match db::bancho::add_room(&mut *tx, match_id, mp_id, bot, bot_made).await {
        Ok(()) => {}
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("match_rooms_pkey") => {
            return Err(RoomError::Taken.into());
        }
        Err(err) => return Err(err.into()),
    }
    // The channel's card lists the rooms; only an unconcluded match announces one.
    let mut jobs = vec![Job::SyncMatch { match_id }];
    if !concluded {
        jobs.push(Job::RoomAdded { match_id, mp_id });
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(())
}
