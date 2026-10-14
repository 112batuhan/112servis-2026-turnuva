use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{
        self,
        bracket::Side,
        notifications::{NotificationKind, NotificationRow, Response, StaffRole},
    },
    discord::jobs::Job,
    error::AppError,
    team_rank, AppState,
};

// Routes under the `/api` nest. Each user reads and answers only their own
// notifications; they're created by the actions they report on (reschedule.rs,
// rank_refresh.rs).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/count", get(notification_count))
        .route("/notifications/read", post(mark_read))
        .route("/notifications/:id/confirm", post(confirm))
        .route("/notifications/:id/decline", post(decline))
}

// How many of the newest notifications the list returns.
const LIST_LIMIT: i64 = 50;

#[derive(Debug, Serialize)]
pub struct Notification {
    id: Uuid,
    kind: NotificationKind,
    // The match it's about; null for rank_over_limit.
    match_id: Option<Uuid>,
    side: Option<Side>,
    number: Option<i32>,
    team1_name: Option<String>,
    team2_name: Option<String>,
    // Who caused it: the player who accepted or proposed the time, the staff member who
    // declined it, or the player whose rank change took the team over the limit.
    actor_username: Option<String>,
    staff_role: Option<StaffRole>,
    #[serde(with = "time::serde::rfc3339::option")]
    old_starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    new_starts_at: Option<OffsetDateTime>,
    response: Option<Response>,
    #[serde(with = "time::serde::rfc3339::option")]
    responded_at: Option<OffsetDateTime>,
    read: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    // reschedule_proposed: the recipient's team; rank_over_limit: the team over the limit.
    team_id: Option<Uuid>,
    team_name: Option<String>,
    // rank_over_limit: the actor's rank before and after, and the team value it led to.
    old_rank: Option<i64>,
    new_rank: Option<i64>,
    team_value: Option<f64>,
    limit: Option<f64>,
    // reschedule_proposed: the opponent's proposals for the match still open.
    pending_proposals: i64,
    // A confirm_time that can still be answered.
    actionable: bool,
}

impl Notification {
    fn from_row(row: NotificationRow) -> Self {
        Self {
            id: row.id,
            kind: row.kind,
            match_id: row.match_id,
            side: row.side,
            number: row.number,
            team1_name: row.team1_name,
            team2_name: row.team2_name,
            actor_username: row.actor_username,
            staff_role: row.staff_role,
            old_starts_at: row.old_starts_at,
            new_starts_at: row.new_starts_at,
            response: row.response,
            responded_at: row.responded_at,
            read: row.read_at.is_some(),
            created_at: row.created_at,
            team_id: row.team_id,
            team_name: row.team_name,
            old_rank: row.old_rank,
            new_rank: row.new_rank,
            team_value: row.team_value,
            limit: (row.kind == NotificationKind::RankOverLimit).then_some(team_rank::LIMIT),
            pending_proposals: row.pending_proposals,
            actionable: row.actionable,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationList {
    notifications: Vec<Notification>,
    // Unread notifications plus confirmations still waiting for an answer.
    attention: i64,
}

// GET /api/notifications — the caller's newest notifications, newest first.
pub async fn list_notifications(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let rows = db::notifications::list(&state.db, user.id, LIST_LIMIT).await?;
    Ok(Json(NotificationList {
        notifications: rows.into_iter().map(Notification::from_row).collect(),
        attention: db::notifications::attention_count(&state.db, user.id).await?,
    }))
}

#[derive(Debug, Serialize)]
pub struct NotificationCount {
    attention: i64,
}

// GET /api/notifications/count — just the bell's number, for polling.
pub async fn notification_count(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(NotificationCount {
        attention: db::notifications::attention_count(&state.db, user.id).await?,
    }))
}

// POST /api/notifications/read — marks all of the caller's notifications read.
// Unanswered confirmations still count until they're answered.
pub async fn mark_read(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    db::notifications::mark_all_read(&state.db, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/notifications/:id/confirm — the match's referee or streamer can make its
// new time.
pub async fn confirm(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    answer_as(&state, user.id, id, Response::Confirmed).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/notifications/:id/decline — the match's referee or streamer can't make its
// new time: they're taken off the match, and everyone who could take their place
// (hosts included) is told.
pub async fn decline(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    answer_as(&state, user.id, id, Response::Declined).await?;
    Ok(StatusCode::NO_CONTENT)
}

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
    const GONE: AppError =
        AppError::Conflict("the match has been moved again since, answer the newer notification");
    let match_id = db::notifications::confirm_match_id(&state.db, id, user_id)
        .await?
        .ok_or(GONE)?;

    let mut tx = state.db.begin().await?;
    let m = db::bracket::lock_match(&mut tx, match_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let n = db::notifications::lock_confirm(&mut tx, id, user_id)
        .await?
        .ok_or(GONE)?;
    if n.response.is_some() {
        return Err(AppError::Conflict("you already answered this"));
    }
    if m.concluded() {
        return Err(AppError::Conflict("this match is concluded"));
    }
    let holder = match n.staff_role {
        StaffRole::Referee => m.referee_id,
        StaffRole::Streamer => m.streamer_id,
    };
    if holder != Some(user_id) {
        return Err(AppError::Conflict("you're no longer on this match"));
    }
    if m.starts_at != Some(n.new_starts_at) {
        return Err(AppError::Conflict("the match's time has changed since"));
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
