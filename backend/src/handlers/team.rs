use axum::{
    extract::{Path, Query, State},
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
    db::{
        self,
        team::{InviteRow, Player, PlayerState, TeamMemberRow},
    },
    error::AppError,
    role::Role,
    team_rank, AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/players/search", get(search_players))
        .route("/me/team", get(my_team_status))
        .route("/invites", post(send_invite))
        .route("/invites/:id", delete(delete_invite))
        .route("/invites/:id/accept", post(accept_invite))
        .route("/public/teams", get(list_public_teams))
        .route("/teams", get(list_teams))
        .route("/teams/:id", delete(disband_team))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Eligibility {
    Eligible,
    NotPlayer,
    InTeam,
    Unranked,
    OverLimit,
}

impl Eligibility {
    fn of(me: &Player, other: &PlayerState) -> Self {
        if other.role != Role::Basic {
            return Self::NotPlayer;
        }
        if other.in_team {
            return Self::InTeam;
        }
        match (me.global_rank, other.player.global_rank) {
            (Some(a), Some(b)) if team_rank::is_allowed(a, b) => Self::Eligible,
            (Some(_), Some(_)) => Self::OverLimit,
            _ => Self::Unranked,
        }
    }

    fn require(self) -> Result<(), AppError> {
        match self {
            Self::Eligible => Ok(()),
            Self::NotPlayer => Err(AppError::BadRequest("staff can't join teams")),
            Self::InTeam => Err(AppError::Conflict("that player is already in a team")),
            Self::Unranked => Err(AppError::BadRequest("both players need a global rank")),
            Self::OverLimit => Err(AppError::BadRequest(
                "this team would be over the rank limit",
            )),
        }
    }
}

fn team_value(a: &Player, b: &Player) -> Option<f64> {
    Some(team_rank::team_value(a.global_rank?, b.global_rank?))
}

#[derive(Debug, Serialize)]
pub struct Team<P> {
    id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    team_value: Option<f64>,
    players: Vec<P>,
}

#[derive(Debug, Serialize)]
pub struct AdminTeamPlayer {
    #[serde(flatten)]
    player: Player,
    discord_username: Option<String>,
}

// Rows arrive ordered by team, so consecutive rows with the same team_id belong together.
fn group_teams<P>(
    rows: Vec<TeamMemberRow>,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Vec<Team<P>> {
    let mut teams: Vec<(Team<P>, Vec<Player>)> = Vec::new();
    for row in rows {
        if teams.last().map(|(t, _)| t.id) != Some(row.team_id) {
            let team = Team {
                id: row.team_id,
                created_at: row.team_created_at,
                team_value: None,
                players: Vec::new(),
            };
            teams.push((team, Vec::new()));
        }
        let (team, players) = teams.last_mut().unwrap();
        players.push(row.player.clone());
        team.players.push(to_player(row));
    }
    teams
        .into_iter()
        .map(|(mut team, players)| {
            if let [a, b] = &players[..] {
                team.team_value = team_value(a, b);
            }
            team
        })
        .collect()
}

// Only basic users play; hosts and map poolers run the tournament. Forming a team
// also needs a linked Discord account.
fn require_player(user: &AuthUser) -> Result<(), AppError> {
    if user.role != Role::Basic {
        return Err(AppError::BadRequest("staff can't join teams"));
    }
    user.require_discord_verified()
}

async fn load_self(state: &AppState, user: &AuthUser) -> Result<PlayerState, AppError> {
    db::team::find_player(&state.db, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(e) if e.code().as_deref() == Some("23505"))
}

// ---------- player search ----------

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    q: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerResult {
    #[serde(flatten)]
    player: Player,
    eligibility: Eligibility,
    team_value: Option<f64>,
    invite_sent: bool,
    invite_received: bool,
}

// GET /api/players/search?q= — players matching a username, with whether the
// caller could team up with each one.
pub async fn search_players(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let me = load_self(&state, &user).await?.player;
    let rows = db::team::search_players(&state.db, user.id, query).await?;
    let results = rows
        .into_iter()
        .map(|row| PlayerResult {
            eligibility: Eligibility::of(&me, &row.state),
            team_value: team_value(&me, &row.state.player),
            invite_sent: row.invite_sent,
            invite_received: row.invite_received,
            player: row.state.player,
        })
        .collect::<Vec<_>>();
    Ok(Json(results))
}

// ---------- the caller's team and invites ----------

#[derive(Debug, Serialize)]
pub struct InviteView {
    id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    player: Player,
    eligibility: Eligibility,
    team_value: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TeamStatus {
    team: Option<Team<Player>>,
    global_rank: Option<i64>,
    min_partner_rank: Option<i64>,
    limit: f64,
    sent: Vec<InviteView>,
    received: Vec<InviteView>,
}

// GET /api/me/team — the caller's team (if any) and their pending invites.
pub async fn my_team_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let me = load_self(&state, &user).await?.player;
    let team = group_teams(
        db::team::list_team_members(&state.db, Some(user.id)).await?,
        |row| row.player,
    )
    .pop();

    let view = |row: InviteRow| InviteView {
        id: row.invite_id,
        created_at: row.created_at,
        eligibility: Eligibility::of(&me, &row.state),
        team_value: team_value(&me, &row.state.player),
        player: row.state.player,
    };
    let sent = db::team::list_sent_invites(&state.db, user.id).await?;
    let received = db::team::list_received_invites(&state.db, user.id).await?;

    Ok(Json(TeamStatus {
        team,
        global_rank: me.global_rank,
        min_partner_rank: me.global_rank.map(team_rank::min_partner_rank),
        limit: team_rank::LIMIT,
        sent: sent.into_iter().map(view).collect(),
        received: received.into_iter().map(view).collect(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SendInviteBody {
    user_id: Uuid,
}

// POST /api/invites — invite another player to team up.
pub async fn send_invite(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SendInviteBody>,
) -> Result<impl IntoResponse, AppError> {
    require_player(&user)?;
    if body.user_id == user.id {
        return Err(AppError::BadRequest("you can't invite yourself"));
    }
    let me = load_self(&state, &user).await?;
    if me.in_team {
        return Err(AppError::Conflict("you are already in a team"));
    }
    let target = db::team::find_player(&state.db, body.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Eligibility::of(&me.player, &target).require()?;

    if db::team::invite_exists(&state.db, body.user_id, user.id).await? {
        return Err(AppError::Conflict(
            "they already invited you, accept their invite instead",
        ));
    }
    db::team::create_invite(&state.db, user.id, body.user_id)
        .await?
        .ok_or(AppError::Conflict("invite already sent"))?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/invites/:id — cancel a sent invite or decline a received one.
pub async fn delete_invite(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if !db::team::delete_invite(&state.db, id, user.id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/invites/:id/accept — forms a team with the inviter. Eligibility is
// checked again here since ranks or teams may have changed since the invite.
pub async fn accept_invite(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_player(&user)?;
    let mut tx = state.db.begin().await?;
    let inviter_id = db::team::lock_received_invite(&mut tx, id, user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    let me = db::team::find_player(&mut *tx, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    let inviter = db::team::find_player(&mut *tx, inviter_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if me.in_team {
        return Err(AppError::Conflict("you are already in a team"));
    }
    Eligibility::of(&me.player, &inviter).require()?;

    match db::team::create_team(&mut tx, inviter_id, user.id).await {
        Ok(_) => {}
        Err(err) if is_unique_violation(&err) => {
            return Err(AppError::Conflict("one of you is already in a team"))
        }
        Err(err) => return Err(err.into()),
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- team lists ----------

// GET /api/public/teams — every team with its players.
pub async fn list_public_teams(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let rows = db::team::list_team_members(&state.db, None).await?;
    Ok(Json(group_teams(rows, |row| row.player)))
}

// GET /api/teams — host-only. Like the public list, plus each player's Discord.
pub async fn list_teams(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let rows = db::team::list_team_members(&state.db, None).await?;
    Ok(Json(group_teams(rows, |row| AdminTeamPlayer {
        player: row.player,
        discord_username: row.discord_username,
    })))
}

// DELETE /api/teams/:id — host-only. Disbands a team; its players become free agents.
pub async fn disband_team(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::team::delete_team(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
