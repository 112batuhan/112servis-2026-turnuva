//! What's done to the bracket, whoever asks: the routes in handlers/bracket.rs and
//! handlers/matches.rs, the room bots (crate::bancho), which save the scores they
//! count, and the changes elsewhere that move its potential matches.

use std::collections::HashSet;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    db::{
        self,
        bracket::{BracketLock, MapResultRow, Outcome},
        matches::MatchKind,
        potential::Before,
    },
    discord::jobs::Job,
    draft::StepKind,
    error::{AppError, GeneralError, MatchError},
    AppState,
};

const SCORE_MAX: i32 = 99;

// What a referee (or the auto referee) marks on a pool map for one of the match's teams.
// A map can't be both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Ban,
    Pick,
}

// Marks a map as banned or picked by one of the match's teams (or moves the mark to the
// other team). The callers have checked the team and the map belong to the match.
pub async fn mark(
    state: &AppState,
    id: Uuid,
    map_id: Uuid,
    team_id: Uuid,
    mark: Mark,
) -> Result<(), AppError> {
    let (marked, taken) = match mark {
        Mark::Ban => (
            db::bracket::set_ban(&state.db, id, map_id, team_id).await?,
            MatchError::MapPicked,
        ),
        Mark::Pick => (
            db::bracket::set_pick(&state.db, id, map_id, team_id).await?,
            MatchError::MapBanned,
        ),
    };
    if !marked {
        return Err(taken.into());
    }
    marks_changed(state, id).await
}

// A team let its ban or pick run out on the auto referee's clock: a ban is lost, a pick
// passes to the other team (draft.rs).
pub async fn miss(
    state: &AppState,
    id: Uuid,
    team_id: Uuid,
    kind: StepKind,
) -> Result<(), AppError> {
    db::bracket::add_missed(&state.db, id, team_id, kind).await?;
    marks_changed(state, id).await
}

// Takes back a ban or pick that ran out: the team has the next turn again, as the
// referee's say until the next mark. When it was the match's first turn that also keeps
// the roll's winner, which nothing else would say once no turn is left.
pub async fn take_back_miss(state: &AppState, id: Uuid, missed_id: Uuid) -> Result<(), AppError> {
    let team_id = db::bracket::delete_missed(&state.db, id, missed_id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    db::referee::set_next_pick(&state.db, id, Some(team_id)).await?;
    state.live.match_changed(id);
    Ok(())
}

// A ban or pick was marked, missed or taken back: the rules say whose pick is next again, and
// the match's pages reload it.
pub async fn marks_changed(state: &AppState, id: Uuid) -> Result<(), AppError> {
    db::referee::clear_next_pick(&state.db, id).await?;
    state.live.match_changed(id);
    Ok(())
}

// What Discord has to hear of a bracket match about to be deleted: its channel goes to
// the archive, and its public announcement, if it had one, stays and says it was
// cancelled.
pub async fn deletion_jobs(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Vec<Job>> {
    let mut jobs = Vec::new();
    if let Some(channel_id) = db::discord::channel_of(&mut *conn, id).await? {
        jobs.push(Job::ArchiveChannel {
            channel_id,
            kind: MatchKind::Bracket,
        });
    }
    let stored = db::discord::match_discord(&mut *conn, id).await?;
    let info = db::discord::match_info(&mut *conn, id).await?;
    if let (Some(channel_id), Some(message_id), Some(info)) =
        (stored.announce_channel_id, stored.announce_message_id, info)
    {
        jobs.push(Job::AnnouncementCancelled {
            channel_id,
            message_id,
            title: crate::discord::announcement_title(&info),
        });
    }
    Ok(jobs)
}
// More finished maps than any match plays, warmups and replays included.
const MAP_RESULTS_MAX: usize = 50;
// Longer than any set of mod acronyms a player can have on.
const MODS_MAX: usize = 40;

// Saves a bracket match's score and, when given, the map results behind it, which
// replace the saved ones (the results post lists them). Checks them the same way
// whoever sends them.
pub async fn save_score(
    state: &AppState,
    id: Uuid,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    maps: Option<&[MapResultRow]>,
) -> Result<(), AppError> {
    let valid = |score: Option<i32>| score.is_none_or(|s| (0..=SCORE_MAX).contains(&s));
    if !valid(team1_score) || !valid(team2_score) {
        return Err(MatchError::BadScore.into());
    }
    if let Some(maps) = maps {
        if maps.len() > MAP_RESULTS_MAX {
            return Err(MatchError::TooManyResults.into());
        }
        if maps.iter().any(|m| m.team1_score < 0 || m.team2_score < 0) {
            return Err(MatchError::NegativeMapScore.into());
        }
        for map in maps {
            let mut seen = HashSet::new();
            for p in &map.players {
                if p.score < 0 || !seen.insert(p.user_id) {
                    return Err(MatchError::BadMapScores.into());
                }
                // Mod acronyms: "HDHR", "NF".
                if p.mods.as_deref().is_some_and(|m| {
                    m.len() > MODS_MAX || !m.chars().all(|c| c.is_ascii_alphanumeric())
                }) {
                    return Err(MatchError::BadMapMods.into());
                }
            }
        }
        let players: Vec<Uuid> = maps
            .iter()
            .flat_map(|m| m.players.iter().map(|p| p.user_id))
            .collect();
        if !db::bracket::players_in_match(&state.db, id, &players).await? {
            return Err(MatchError::PlayerNotInMatch.into());
        }
        let Some(ids) = maps.iter().map(|m| m.map_id).collect::<Option<Vec<Uuid>>>() else {
            return Err(MatchError::ResultNeedsMap.into());
        };
        if !db::bracket::maps_in_pool(&state.db, id, &ids).await? {
            return Err(MatchError::MapNotInPool.into());
        }
    }
    // The match card in its Discord channel shows the running score, so it's synced
    // with every change; the open pages hear it right away.
    let mut tx = state.db.begin().await?;
    if !db::bracket::set_score(&mut *tx, id, team1_score, team2_score).await? {
        return Err(MatchError::ConcludedScore.into());
    }
    if let Some(maps) = maps {
        db::bracket::replace_map_results(&mut tx, id, maps).await?;
    }
    let mut jobs = vec![Job::SyncMatch { match_id: id }];
    // A match being played that never got its public announcement (its time passed
    // before the scheduler's window) gets it now, with the score in it.
    if state.discord.enabled() && db::discord::claim_late_announcement(&mut tx, id).await? {
        jobs.push(Job::Announce { match_id: id });
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.score(id, team1_score, team2_score);
    Ok(())
}

// The winner and loser of a match with both teams: the team that didn't forfeit, or
// else the one ahead on a score that isn't a tie.
fn outcome(m: &BracketLock) -> Option<(Uuid, Uuid)> {
    let (t1, t2) = (m.team1_id?, m.team2_id?);
    match m.forfeit_slot {
        Some(1) => return Some((t2, t1)),
        Some(_) => return Some((t1, t2)),
        None => {}
    }
    match m.team1_score?.cmp(&m.team2_score?) {
        std::cmp::Ordering::Greater => Some((t1, t2)),
        std::cmp::Ordering::Less => Some((t2, t1)),
        std::cmp::Ordering::Equal => None,
    }
}

// Concludes a bracket match and puts its winner and loser into the slots they're
// linked to. Called by the conclude route once it has checked who's asking, and by
// `forfeit` with the slot that forfeited, which needs neither a room nor a score.
pub async fn conclude(state: &AppState, id: Uuid, forfeit: Option<i16>) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let mut m = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    if m.concluded() {
        return Err(MatchError::Concluded.into());
    }
    if forfeit.is_none() && !m.has_room {
        return Err(MatchError::NeedsRoom.into());
    }
    if m.team1_id.is_none() || m.team2_id.is_none() {
        return Err(MatchError::NeedsBothTeams.into());
    }
    m.forfeit_slot = forfeit;
    let (winner, loser) = outcome(&m).ok_or(MatchError::NeedsWinner)?;
    if forfeit.is_some() {
        db::bracket::set_forfeit(&mut tx, id, forfeit).await?;
    }
    // The channel is archived, the results posted, and the matches the teams move on to
    // may now have both teams and so their channel.
    let mut jobs = vec![
        Job::SyncMatch { match_id: id },
        Job::Concluded { match_id: id },
        Job::PostResults { match_id: id },
    ];
    for target in db::bracket::lock_targets(&mut tx, id).await? {
        if target.concluded_at.is_some() {
            return Err(MatchError::NextConcluded.into());
        }
        let team = match target.outcome {
            Outcome::Winner => winner,
            Outcome::Loser => loser,
        };
        db::bracket::set_slot(&mut tx, target.target_id, target.slot, Some(team)).await?;
        jobs.push(Job::SyncMatch {
            match_id: target.target_id,
        });
    }
    db::reschedule::delete_for_match(&mut tx, id).await?;
    let concluded = match forfeit {
        Some(_) => db::matches::conclude_forfeit(&mut *tx, id).await?,
        None => db::matches::conclude(&mut *tx, id).await?,
    };
    // The row is locked, so this only fails if the room was removed in between.
    if !concluded {
        return Err(MatchError::Changed.into());
    }
    // The pairings its teams were in narrow down: the loser's go, and a match that now
    // has both teams takes over its own.
    jobs.extend(db::potential::sync(&mut tx, &[]).await?);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}

// Reopens a bracket match and takes its teams back out of the slots it put them in,
// unless a host has put another team there since.
pub async fn reopen(state: &AppState, id: Uuid) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let m = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    if !m.concluded() {
        return Ok(());
    }
    let result = outcome(&m);
    let mut jobs = vec![
        Job::SyncMatch { match_id: id },
        Job::Reopened { match_id: id },
        Job::DeleteResults { match_id: id },
    ];
    let mut before = Vec::new();
    for target in db::bracket::lock_targets(&mut tx, id).await? {
        if target.concluded_at.is_some() {
            return Err(MatchError::NextConcluded.into());
        }
        let Some((winner, loser)) = result else {
            continue;
        };
        let team = match target.outcome {
            Outcome::Winner => winner,
            Outcome::Loser => loser,
        };
        let current = if target.slot == 1 {
            target.team1_id
        } else {
            target.team2_id
        };
        if current == Some(team) {
            before.push(Before {
                match_id: target.target_id,
                team1_id: target.team1_id,
                team2_id: target.team2_id,
            });
            db::bracket::set_slot(&mut tx, target.target_id, target.slot, None).await?;
            jobs.push(Job::SyncMatch {
                match_id: target.target_id,
            });
        }
    }
    db::bracket::set_forfeit(&mut tx, id, None).await?;
    db::matches::reopen(&mut *tx, id).await?;
    // The matches it decided go back to their pairings.
    jobs.extend(db::potential::sync(&mut tx, &before).await?);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}

// Works the potential matches out again after a change that doesn't go through the
// bracket's own routes (a stage deleted, a dev reseed) and at startup.
pub async fn resync_potentials(state: &AppState) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let jobs = db::potential::sync(&mut tx, &[]).await?;
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}
