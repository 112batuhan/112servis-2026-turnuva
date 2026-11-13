// Keeps stored ranks in step with osu!, through the bulk users endpoint.
use std::{collections::HashMap, time::Duration as StdDuration};

use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    db::{self, ranks::RankChange},
    discord::jobs::Job,
    error::{AppError, TeamError},
    osu_api::{self, user::USERS_PER_REQUEST},
    team_rank, AppState,
};

// Ranks pulled more recently than this are trusted as they are.
const FRESH_FOR: Duration = Duration::minutes(10);
// How often team members are refreshed, and how often the task checks for it.
const FULL_REFRESH_EVERY: Duration = Duration::days(1);
const CHECK_INTERVAL: StdDuration = StdDuration::from_secs(60 * 60);

async fn refresh(state: &AppState, osu_ids: &[i64]) -> Result<u64, AppError> {
    if osu_ids.is_empty() {
        return Ok(0);
    }
    let token =
        osu_api::token::app_token(&state.osu_client, &state.oauth_http, &state.osu_app_token)
            .await?;
    let mut changes = Vec::new();
    for chunk in osu_ids.chunks(USERS_PER_REQUEST) {
        let users = osu_api::user::fetch_many(&state.http_client, &token, chunk).await?;
        changes.extend(db::ranks::update_from_osu(&state.db, &users).await?);
    }
    notify_over_limit(state, &changes).await;
    Ok(changes.len() as u64)
}

// Tells a team's players and the hosts when rank changes take the team over the
// limit. Only a team that was within it before counts, so a team over the limit isn't
// told again on every refresh. The member whose value rose the most is named as the
// cause. A failure is only logged: the ranks are already saved.
pub async fn notify_over_limit(state: &AppState, changes: &[RankChange]) {
    let changed: HashMap<Uuid, RankChange> = changes
        .iter()
        .filter(|c| c.old_rank != c.new_rank)
        .map(|c| (c.user_id, *c))
        .collect();
    if changed.is_empty() {
        return;
    }
    let result = async {
        let ids: Vec<Uuid> = changed.keys().copied().collect();
        let rows = db::ranks::team_ranks_of(&state.db, &ids).await?;
        for team in rows.chunk_by(|a, b| a.team_id == b.team_id) {
            let [a, b] = team else { continue };
            let ranks = |m: &db::ranks::TeamRankRow| match changed.get(&m.user_id) {
                Some(c) => (c.old_rank, m.global_rank),
                None => (m.global_rank, m.global_rank),
            };
            let Some(i) = over_limit_cause([ranks(a), ranks(b)]) else {
                continue;
            };
            let cause = [a, b][i];
            let (old_rank, new_rank) = ranks(cause);
            let (Some(rank_a), Some(rank_b)) = (a.global_rank, b.global_rank) else {
                continue;
            };
            let team_value = team_rank::team_value(rank_a, rank_b);
            let notified = db::notifications::notify_over_limit(
                &state.db,
                a.team_id,
                cause.user_id,
                old_rank,
                new_rank,
                team_value,
            )
            .await?;
            state
                .discord
                .enqueue(
                    &state.db,
                    &[Job::RankOverLimit {
                        team_id: a.team_id,
                        user_id: cause.user_id,
                        old_rank,
                        new_rank,
                        team_value,
                    }],
                )
                .await?;
            state.live.notifications_changed(notified);
        }
        Ok::<_, sqlx::Error>(())
    }
    .await;
    if let Err(err) = result {
        tracing::error!(error = ?err, "could not send rank limit notifications");
    }
}

// Given both members' (old, new) ranks, which one took the team over the limit, if
// the change did: the team must be within the limit before (or unranked) and over it
// after. Of two members who changed, it's the one whose value rose the most; one who
// gets a rank after being unranked counts as the biggest rise.
fn over_limit_cause(members: [(Option<i64>, Option<i64>); 2]) -> Option<usize> {
    let [(old_a, new_a), (old_b, new_b)] = members;
    let was_over = matches!((old_a, old_b), (Some(x), Some(y)) if !team_rank::is_allowed(x, y));
    let now_over = matches!((new_a, new_b), (Some(x), Some(y)) if !team_rank::is_allowed(x, y));
    if was_over || !now_over {
        return None;
    }
    let rise = |(old, new): (Option<i64>, Option<i64>)| match (old, new) {
        _ if old == new => f64::NEG_INFINITY,
        (Some(old), Some(new)) => team_rank::player_value(new) - team_rank::player_value(old),
        (None, Some(_)) => f64::INFINITY,
        _ => f64::NEG_INFINITY,
    };
    Some(usize::from(rise(members[1]) > rise(members[0])))
}

// Refreshes whichever of the users have a rank older than FRESH_FOR and returns
// how many changed. A failure is only logged, so callers carry on with the stored
// ranks. Frozen teams keep the ranks they were frozen with.
pub async fn refresh_stale(state: &AppState, user_ids: &[Uuid]) -> u64 {
    let result = async {
        if db::settings::teams_frozen(&state.db).await? {
            return Ok(0);
        }
        let cutoff = OffsetDateTime::now_utc() - FRESH_FOR;
        let ids = db::ranks::stale_osu_ids(&state.db, user_ids, cutoff).await?;
        refresh(state, &ids).await
    }
    .await;
    result.unwrap_or_else(|err| {
        tracing::warn!(error = ?err, "rank refresh failed, using stored ranks");
        0
    })
}

pub struct FullRefresh {
    pub updated: u64,
    pub at: OffsetDateTime,
}

// Refreshes every team member. Runs daily and from the admin panel; only one runs
// at a time.
pub async fn refresh_team_members(state: &AppState) -> Result<FullRefresh, AppError> {
    let _running = state
        .rank_refresh_lock
        .try_lock()
        .map_err(|_| TeamError::RankUpdateRunning)?;
    if db::settings::teams_frozen(&state.db).await? {
        return Err(TeamError::Frozen.into());
    }
    let ids = db::ranks::team_member_osu_ids(&state.db).await?;
    let updated = refresh(state, &ids).await?;
    let at = db::settings::touch_ranks_updated_at(&state.db).await?;
    tracing::info!("refreshed ranks of {updated} team members");
    Ok(FullRefresh { updated, at })
}

// Checks hourly whether the last full refresh is a day old, so restarts neither
// skip nor repeat it. Nothing runs while teams are frozen.
pub fn spawn_daily_refresh(state: AppState) {
    crate::supervise::spawn("daily rank refresh", move || daily_refresh(state.clone()));
}

async fn daily_refresh(state: AppState) {
    {
        let mut interval = tokio::time::interval(CHECK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let due = async {
                if db::settings::teams_frozen(&state.db).await? {
                    return Ok(false);
                }
                let last = db::settings::ranks_updated_at(&state.db).await?;
                Ok::<_, sqlx::Error>(
                    last.is_none_or(|at| OffsetDateTime::now_utc() - at >= FULL_REFRESH_EVERY),
                )
            };
            let due = match due.await {
                Ok(due) => due,
                Err(err) => {
                    tracing::error!(error = ?err, "could not read the last rank refresh");
                    continue;
                }
            };
            if due {
                if let Err(err) = refresh_team_members(&state).await {
                    tracing::error!(error = ?err, "daily rank refresh failed");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A pair right at the limit: `edge` with the best partner rank it may have.
    const EDGE: i64 = 6400;

    #[test]
    fn going_over_names_the_player_who_improved() {
        let min = team_rank::min_partner_rank(EDGE);
        let before = [(Some(EDGE), Some(EDGE)), (Some(min), Some(min - 50))];
        assert_eq!(over_limit_cause(before), Some(1));
        let swapped = [(Some(min), Some(min - 50)), (Some(EDGE), Some(EDGE))];
        assert_eq!(over_limit_cause(swapped), Some(0));
    }

    #[test]
    fn staying_within_or_staying_over_is_quiet() {
        let min = team_rank::min_partner_rank(EDGE);
        assert_eq!(
            over_limit_cause([(Some(EDGE), Some(EDGE)), (Some(min + 50), Some(min))]),
            None
        );
        assert_eq!(
            over_limit_cause([(Some(EDGE), Some(EDGE)), (Some(min - 10), Some(min - 60))]),
            None
        );
    }

    #[test]
    fn both_changing_names_the_bigger_rise() {
        let min = team_rank::min_partner_rank(EDGE);
        let members = [(Some(EDGE), Some(EDGE - 5)), (Some(min), Some(min - 500))];
        assert_eq!(over_limit_cause(members), Some(1));
    }

    #[test]
    fn getting_a_rank_counts() {
        assert_eq!(
            over_limit_cause([(Some(100), Some(100)), (None, Some(100))]),
            Some(1)
        );
        assert_eq!(
            over_limit_cause([(Some(100), None), (Some(100), Some(100))]),
            None
        );
    }
}
