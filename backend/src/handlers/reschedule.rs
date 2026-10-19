use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{self, bracket::BracketLock, bracket::Side, reschedule::ProposalRow, team::Player},
    discord::jobs::Job,
    error::AppError,
    AppState,
};

// Routes under the `/api` nest. The two teams of a bracket match move it themselves:
// either one proposes times, and a player of the other team accepts one. Like team
// invites, but between teams, and `teams_frozen` doesn't affect it. The teams of a
// potential match agree on its time the same way, before they know they'll play it.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me/reschedules", get(list_my_reschedules))
        .route("/reschedules", post(propose))
        .route("/reschedules/:id", delete(delete_proposal))
        .route("/reschedules/:id/accept", post(accept))
}

// The most times a team can have proposed for one match at once.
const PROPOSAL_LIMIT: i64 = 10;

// Players move a match only until it starts: once its time has come or a referee has
// opened its room, rescheduling it is up to the staff.
fn is_open(starts_at: Option<OffsetDateTime>, has_room: bool, now: OffsetDateTime) -> bool {
    !has_room && starts_at.is_none_or(|at| at > now)
}

fn require_open(m: &BracketLock) -> Result<(), AppError> {
    if m.concluded() {
        return Err(AppError::Conflict("this match is concluded"));
    }
    if !is_open(m.starts_at, m.has_room, OffsetDateTime::now_utc()) {
        return Err(AppError::Conflict(
            "this match has started, only the staff can move it now",
        ));
    }
    Ok(())
}

fn plays(m: &BracketLock, team_id: Uuid) -> bool {
    m.team1_id == Some(team_id) || m.team2_id == Some(team_id)
}

async fn my_team_id(state: &AppState, user: &AuthUser) -> Result<Uuid, AppError> {
    db::team::find_team_id(&state.db, user.id)
        .await?
        .ok_or(AppError::BadRequest("you aren't in a team"))
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    proposed_by: Player,
}

impl Proposal {
    fn from_row(row: ProposalRow) -> Self {
        Self {
            id: row.proposal_id,
            starts_at: row.starts_at,
            created_at: row.created_at,
            proposed_by: row.proposed_by,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Opponent {
    id: Uuid,
    name: String,
}

#[derive(Debug, Serialize)]
pub struct RescheduleMatch {
    id: Uuid,
    // A pairing its bracket match may still turn out to be (side and number are that
    // match's): played only if both teams get there.
    potential: bool,
    side: Side,
    number: i32,
    stage_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    opponent: Opponent,
    // Times can still be proposed and accepted: the match hasn't started.
    open: bool,
    // Proposed by the caller's team, for the opponent to accept.
    sent: Vec<Proposal>,
    // Proposed by the opponent, for the caller's team to accept.
    received: Vec<Proposal>,
}

#[derive(Debug, Serialize)]
pub struct MyReschedules {
    matches: Vec<RescheduleMatch>,
    limit: i64,
}

// GET /api/me/reschedules — the caller's team's unconcluded matches that have both
// teams, each with the times either side has proposed for it. Empty without a team.
pub async fn list_my_reschedules(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let Some(team_id) = db::team::find_team_id(&state.db, user.id).await? else {
        return Ok(Json(MyReschedules {
            matches: Vec::new(),
            limit: PROPOSAL_LIMIT,
        }));
    };
    let rows = db::reschedule::list_team_matches(&state.db, team_id).await?;
    let mut proposals = db::reschedule::list_team_proposals(&state.db, team_id).await?;
    let now = OffsetDateTime::now_utc();
    let matches = rows
        .into_iter()
        .map(|row| {
            let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut proposals)
                .into_iter()
                .partition(|p| p.match_id == row.id);
            proposals = rest;
            let (sent, received): (Vec<_>, Vec<_>) =
                mine.into_iter().partition(|p| p.team_id == team_id);
            RescheduleMatch {
                id: row.id,
                potential: row.potential,
                side: row.side,
                number: row.number,
                stage_name: row.stage_name,
                starts_at: row.starts_at,
                opponent: Opponent {
                    id: row.opponent_id,
                    name: row.opponent_name,
                },
                open: is_open(row.starts_at, row.has_room, now),
                sent: sent.into_iter().map(Proposal::from_row).collect(),
                received: received.into_iter().map(Proposal::from_row).collect(),
            }
        })
        .collect();
    Ok(Json(MyReschedules {
        matches,
        limit: PROPOSAL_LIMIT,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ProposeBody {
    match_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
}

// POST /api/reschedules — a player proposes a new time for one of their team's
// matches, and the opponent's players are notified. The match row is locked so the
// limit holds and an accept can't slip in.
pub async fn propose(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ProposeBody>,
) -> Result<impl IntoResponse, AppError> {
    let team_id = my_team_id(&state, &user).await?;
    if body.starts_at <= OffsetDateTime::now_utc() {
        return Err(AppError::BadRequest("pick a time in the future"));
    }

    let mut tx = state.db.begin().await?;
    let m = db::potential::lock_schedulable(&mut tx, body.match_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !plays(&m, team_id) {
        return Err(AppError::NotFound);
    }
    let (Some(team1_id), Some(team2_id)) = (m.team1_id, m.team2_id) else {
        return Err(AppError::Conflict(
            "this match doesn't have its opponent yet",
        ));
    };
    require_open(&m)?;
    if m.starts_at == Some(body.starts_at) {
        return Err(AppError::BadRequest("the match is already at that time"));
    }
    let opponent_id = if team1_id == team_id {
        team2_id
    } else {
        team1_id
    };
    if db::reschedule::proposal_exists(&mut tx, body.match_id, opponent_id, body.starts_at).await? {
        return Err(AppError::Conflict(
            "they already proposed that time, accept theirs instead",
        ));
    }
    if db::reschedule::count_proposals(&mut tx, body.match_id, team_id).await? >= PROPOSAL_LIMIT {
        return Err(AppError::Conflict(
            "your team has too many proposals for this match, cancel some first",
        ));
    }
    db::reschedule::create_proposal(&mut tx, body.match_id, team_id, user.id, body.starts_at)
        .await?
        .ok_or(AppError::Conflict("your team already proposed that time"))?;
    db::notifications::notify_proposed(&mut tx, body.match_id, opponent_id, user.id).await?;
    state
        .discord
        .enqueue(
            &mut *tx,
            &[Job::ProposalMade {
                match_id: body.match_id,
                team_id,
                starts_at: body.starts_at,
            }],
        )
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/reschedules/:id — either team's player cancels their team's proposal
// or declines the opponent's.
pub async fn delete_proposal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let team_id = my_team_id(&state, &user).await?;
    if !db::reschedule::delete_proposal(&state.db, id, team_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/reschedules/:id/accept — a player of the other team accepts a proposed
// time: the match moves there and every other proposal for it, from either side, is
// dropped. Its referee and streamer are asked to confirm the new time and the hosts
// are told (handlers/notifications.rs). The match is locked before the proposal is
// read again, so of two accepts at once the second finds its proposal gone.
pub async fn accept(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let team_id = my_team_id(&state, &user).await?;
    let match_id = db::reschedule::find_proposal(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?
        .match_id;

    let mut tx = state.db.begin().await?;
    let m = db::potential::lock_schedulable(&mut tx, match_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let proposal = db::reschedule::find_proposal(&mut *tx, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !plays(&m, team_id) || !plays(&m, proposal.team_id) {
        return Err(AppError::NotFound);
    }
    if proposal.team_id == team_id {
        return Err(AppError::BadRequest(
            "your team proposed this time, the other team has to accept it",
        ));
    }
    require_open(&m)?;
    if proposal.starts_at <= OffsetDateTime::now_utc() {
        return Err(AppError::Conflict("that time has already passed"));
    }
    db::reschedule::reschedule(&mut tx, match_id, proposal.starts_at).await?;
    db::notifications::notify_rescheduled(
        &mut tx,
        match_id,
        user.id,
        m.referee_id,
        m.streamer_id,
        m.starts_at,
        proposal.starts_at,
    )
    .await?;
    // A match without a time had no channel yet.
    state
        .discord
        .enqueue(
            &mut *tx,
            &[
                Job::SyncMatch { match_id },
                Job::Rescheduled {
                    match_id,
                    actor_id: user.id,
                    old_starts_at: m.starts_at,
                    new_starts_at: proposal.starts_at,
                },
            ],
        )
        .await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}
