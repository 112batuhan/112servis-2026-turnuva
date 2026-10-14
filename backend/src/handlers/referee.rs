use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
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
        chat::ChatMessage,
        mappool::{Category, PublicPoolMap, PublicSlot},
        matches::MatchKind,
    },
    error::AppError,
    handlers::{
        matches::{mp_url, Referee},
        team::{group_teams, AdminTeamPlayer, Team},
    },
    role::Role,
    AppState,
};

// Routes under the `/api` nest. The referee page reads a match from here and runs
// the osu! room over IRC (handlers/irc.rs); changes to the match itself still go
// through the shared `/api/matches/:id/*` routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/matches/:id/room", get(get_room))
        .route("/matches/:id/chat", get(get_chat))
}

#[derive(Debug, Serialize)]
pub struct RoomStage {
    id: Uuid,
    name: String,
}

#[derive(Debug, Serialize)]
pub struct RefereeRoom {
    id: Uuid,
    kind: MatchKind,
    // Bracket matches only: shown as W1, L3.
    side: Option<Side>,
    number: Option<i32>,
    // Qualifiers only: the lobby's place by start time.
    lobby_number: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    referee: Option<Referee>,
    mp_id: Option<i64>,
    mp_url: Option<String>,
    referee_notes: String,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
    // Bracket matches only: which of `teams` plays in which slot (red, blue).
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    // A bracket match's teams, or a lobby's in signup order.
    teams: Vec<Team<AdminTeamPlayer>>,
    // The map pool it's played in, published or not; no editor notes.
    stage: Option<RoomStage>,
    categories: Vec<Category>,
    slots: Vec<PublicSlot>,
    maps: Vec<PublicPoolMap>,
}

// GET /api/matches/:id/room — referees and up. Everything the referee page shows
// about a match: its teams with their Discords, and its map pool.
pub async fn get_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let row = db::matches::find_room(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let members = db::team::list_team_members(&state.db, None).await?;
    let mut all = group_teams(members, AdminTeamPlayer::from_row);
    let order: Vec<Uuid> = match row.kind {
        MatchKind::Bracket => [row.team1_id, row.team2_id].into_iter().flatten().collect(),
        MatchKind::Qualifier => db::matches::list_entries(&state.db)
            .await?
            .into_iter()
            .filter(|e| e.match_id == id)
            .map(|e| e.team_id)
            .collect(),
    };
    let teams = order
        .into_iter()
        .filter_map(|team_id| {
            let i = all.iter().position(|t| t.id == team_id)?;
            Some(all.swap_remove(i))
        })
        .collect();

    let (stage, categories, slots, maps) = match (row.stage_id, row.stage_name) {
        (Some(stage_id), Some(name)) => (
            Some(RoomStage { id: stage_id, name }),
            db::mappool::list_categories(&state.db, stage_id).await?,
            db::mappool::list_public_slots(&state.db, stage_id).await?,
            db::mappool::list_public_maps(&state.db, stage_id).await?,
        ),
        _ => (None, Vec::new(), Vec::new(), Vec::new()),
    };

    Ok(Json(RefereeRoom {
        id: row.id,
        kind: row.kind,
        side: row.side,
        number: row.number,
        lobby_number: row.lobby_number,
        starts_at: row.starts_at,
        referee: Referee::from_columns(row.referee),
        mp_id: row.mp_id,
        mp_url: row.mp_id.map(mp_url),
        referee_notes: row.referee_notes,
        concluded_at: row.concluded_at,
        team1_id: row.team1_id,
        team2_id: row.team2_id,
        team1_score: row.team1_score,
        team2_score: row.team2_score,
        teams,
        stage,
        categories,
        slots,
        maps,
    }))
}

#[derive(Debug, Serialize)]
pub struct RoomChat {
    mp_id: Option<i64>,
    messages: Vec<ChatMessage>,
}

// GET /api/matches/:id/chat — referees and up. Everything said in the match's room
// channel while a referee's relay was in it, oldest first. Empty until it has a room.
pub async fn get_chat(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let m = db::matches::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let messages = match m.mp_id {
        Some(mp_id) => db::chat::list_messages(&state.db, mp_id).await?,
        None => Vec::new(),
    };
    Ok(Json(RoomChat {
        mp_id: m.mp_id,
        messages,
    }))
}
