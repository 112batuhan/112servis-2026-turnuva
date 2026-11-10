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
    error::AppError,
    services::notifications::answer_as,
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
    // The match is a pairing its bracket match may turn out to be (side and number are
    // that match's).
    potential: bool,
    // The bracket match to open for it: the match, or the one it's a pairing of.
    bracket_match_id: Option<Uuid>,
    side: Option<Side>,
    number: Option<i32>,
    team1_name: Option<String>,
    team2_name: Option<String>,
    // Who caused it: the player who accepted or proposed the time, the staff member who
    // declined it, or the player whose rank change took the team over the limit.
    actor_username: Option<String>,
    // confirm_time: a host moved the match in the bracket editor, rather than its teams.
    // Players can't be hosts, so the actor's role tells which.
    by_host: bool,
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
            potential: row.potential,
            bracket_match_id: row.bracket_match_id,
            side: row.side,
            number: row.number,
            team1_name: row.team1_name,
            team2_name: row.team2_name,
            actor_username: row.actor_username,
            by_host: row.kind == NotificationKind::ConfirmTime && row.actor_is_host,
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
async fn list_notifications(
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
async fn notification_count(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(NotificationCount {
        attention: db::notifications::attention_count(&state.db, user.id).await?,
    }))
}

// POST /api/notifications/read — marks all of the caller's notifications read.
// Unanswered confirmations still count until they're answered.
async fn mark_read(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    db::notifications::mark_all_read(&state.db, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/notifications/:id/confirm — the match's referee or streamer can make its
// new time.
async fn confirm(
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
async fn decline(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    answer_as(&state, user.id, id, Response::Declined).await?;
    Ok(StatusCode::NO_CONTENT)
}
