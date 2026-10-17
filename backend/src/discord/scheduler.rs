//! Messages tied to the clock: a reminder in a match's channel an hour before it
//! starts, and the public announcement 15 minutes before a bracket match. A match moved
//! after either went out gets it again for its new time. Also archives the channel of
//! a match an hour after it was concluded.

use std::time::Duration;

use super::jobs::Job;
use crate::{db, AppState};

const CHECK_EVERY: Duration = Duration::from_secs(30);
const REMIND_BEFORE_SECS: i64 = 60 * 60;
const ANNOUNCE_BEFORE_SECS: i64 = 15 * 60;

pub async fn run(state: AppState) {
    let mut interval = tokio::time::interval(CHECK_EVERY);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        // Switched off: nothing is marked sent, so what's still due goes out once it's
        // back on.
        if !state.discord.enabled() {
            continue;
        }
        if let Err(err) = tick(&state).await {
            tracing::error!(error = %err, "discord scheduler failed");
        }
    }
}

// Marks what's due as sent and queues it in one transaction, so nothing goes out twice.
async fn tick(state: &AppState) -> sqlx::Result<()> {
    let mut tx = state.db.begin().await?;
    let mut jobs: Vec<Job> = db::discord::take_due_reminders(&mut tx, REMIND_BEFORE_SECS)
        .await?
        .into_iter()
        .map(|match_id| Job::Reminder { match_id })
        .collect();
    jobs.extend(
        db::discord::take_due_announcements(&mut tx, ANNOUNCE_BEFORE_SECS)
            .await?
            .into_iter()
            .map(|match_id| Job::Announce { match_id }),
    );
    jobs.extend(
        db::discord::take_due_archives(&mut tx)
            .await?
            .into_iter()
            .map(|match_id| Job::SyncMatch { match_id }),
    );
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await
}
