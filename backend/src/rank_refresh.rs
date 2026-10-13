// Keeps stored ranks in step with osu!, through the bulk users endpoint.
use std::time::Duration as StdDuration;

use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    db,
    error::AppError,
    osu_api::{self, user::USERS_PER_REQUEST},
    AppState,
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
    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let mut updated = 0;
    for chunk in osu_ids.chunks(USERS_PER_REQUEST) {
        let users = osu_api::user::fetch_many(&state.http_client, &token, chunk).await?;
        updated += db::ranks::update_from_osu(&state.db, &users).await?;
    }
    Ok(updated)
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
        .map_err(|_| AppError::Conflict("a rank update is already running"))?;
    if db::settings::teams_frozen(&state.db).await? {
        return Err(AppError::Conflict("teams are frozen"));
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
    tokio::spawn(async move {
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
    });
}
