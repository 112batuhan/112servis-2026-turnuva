//! What's done to any match, whoever asks: the routes in handlers/matches.rs and the
//! room bots (crate::bancho), which put the rooms they make on their matches.

use uuid::Uuid;

use crate::{
    db::{
        self,
        matches::{MatchKind, MatchState},
    },
    discord::jobs::Job,
    error::{AppError, GeneralError, MatchError, RoomError},
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
// pages. `bot` runs it; `bot_made` when that bot made it. `announce_bot`: the auto
// referee made it, and the match channel names the bot players can ask for an invite.
pub async fn link_room(
    state: &AppState,
    match_id: Uuid,
    mp_id: i64,
    bot: Option<Uuid>,
    bot_made: bool,
    concluded: bool,
    announce_bot: Option<String>,
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
        jobs.push(Job::RoomAdded {
            match_id,
            mp_id,
            bot: announce_bot,
        });
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(())
}

// Concludes a qualifier lobby. The callers checked it has a room and isn't concluded.
pub async fn conclude_lobby(state: &AppState, id: Uuid) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    if !db::matches::conclude(&mut *tx, id).await? {
        return Err(MatchError::Changed.into());
    }
    state
        .discord
        .enqueue(
            &mut *tx,
            &[
                Job::SyncMatch { match_id: id },
                Job::Concluded { match_id: id },
            ],
        )
        .await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(())
}

// Takes a team out of its qualifier lobby; returns the lobby. Its players lose the
// lobby's channel.
pub async fn remove_lobby_team(state: &AppState, team_id: Uuid) -> Result<Uuid, AppError> {
    let mut tx = state.db.begin().await?;
    let (match_id, _) = db::matches::lock_team_lobby(&mut tx, team_id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    db::matches::delete_entry(&mut *tx, team_id).await?;
    state
        .discord
        .enqueue(&mut *tx, &[Job::SyncMatch { match_id }])
        .await?;
    tx.commit().await?;
    state.live.match_changed(match_id);
    Ok(match_id)
}
