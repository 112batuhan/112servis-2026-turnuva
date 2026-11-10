//! Answering a notification that asks something: a match's referee or streamer saying
//! whether they can make its new time. Asked from the site's bell
//! (handlers/notifications.rs) and from the buttons in the bot's direct message
//! (discord/commands.rs).

use uuid::Uuid;

use crate::{
    db::{
        self,
        notifications::{Response, StaffRole},
    },
    discord::jobs::Job,
    error::{AppError, GeneralError, MatchError, ScheduleError},
    AppState,
};

// The match is locked before the notification, like accepting a reschedule does, so the
// two can't deadlock, and an answer can't land on a time that just changed. Also
// answered from the buttons in the bot's direct message (discord/commands.rs).
pub async fn answer_as(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    response: Response,
) -> Result<(), AppError> {
    // A question nobody answered is replaced when the match is moved again.
    const GONE: ScheduleError = ScheduleError::MovedAgain;
    let match_id = db::notifications::confirm_match_id(&state.db, id, user_id)
        .await?
        .ok_or(GONE)?;

    let mut tx = state.db.begin().await?;
    let m = db::potential::lock_schedulable(&mut tx, match_id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    let n = db::notifications::lock_confirm(&mut tx, id, user_id)
        .await?
        .ok_or(GONE)?;
    if n.response.is_some() {
        return Err(ScheduleError::AlreadyAnswered.into());
    }
    if m.concluded() {
        return Err(MatchError::Concluded.into());
    }
    let holder = match n.staff_role {
        StaffRole::Referee => m.referee_id,
        StaffRole::Streamer => m.streamer_id,
    };
    if holder != Some(user_id) {
        return Err(ScheduleError::NoLongerOnMatch.into());
    }
    if m.starts_at != Some(n.new_starts_at) {
        return Err(ScheduleError::TimeChanged.into());
    }

    db::notifications::respond(&mut tx, id, response).await?;
    let declined = response == Response::Declined;
    if declined {
        db::notifications::clear_staff(&mut tx, n.match_id, n.staff_role).await?;
        db::notifications::notify_declined(
            &mut tx,
            n.match_id,
            user_id,
            n.staff_role,
            n.old_starts_at,
            n.new_starts_at,
        )
        .await?;
    }
    let answered = Job::StaffAnswered {
        match_id: n.match_id,
        user_id,
        role: n.staff_role,
        declined,
    };
    let jobs = if declined {
        vec![
            Job::SyncMatch {
                match_id: n.match_id,
            },
            answered,
        ]
    } else {
        vec![answered]
    };
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}
