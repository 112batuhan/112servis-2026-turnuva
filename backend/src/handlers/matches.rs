use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{
        self,
        matches::{MatchKind, MatchRow, MatchState, RefereeColumns},
        team::{Player, TeamMemberRow},
    },
    error::AppError,
    handlers::{
        bracket,
        team::{group_teams, AdminTeamPlayer, Team},
    },
    role::Role,
    AppState,
};

// Routes under the `/api` nest. Lobbies are created and managed by hosts; any staff
// member can see the staff view, and each match is run by its referee (or a host).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/qualifiers", get(list_public_lobbies))
        .route("/me/qualifier", put(join_lobby).delete(leave_lobby))
        .route("/qualifiers", get(list_lobbies).post(create_lobby))
        .route(
            "/qualifiers/:id",
            patch(reschedule_lobby).delete(delete_lobby),
        )
        .route(
            "/teams/:id/qualifier",
            put(assign_team).delete(unassign_team),
        )
        .route("/referees", get(list_referees))
        .route("/matches/:id/referee", put(set_referee))
        .route("/matches/:id/rooms", post(add_room))
        .route("/matches/:id/rooms/:mp_id", delete(remove_room))
        .route("/matches/:id/notes", put(set_notes))
        .route("/matches/:id/conclude", post(conclude))
        .route("/matches/:id/reopen", post(reopen))
}

// The most teams a qualifier lobby holds.
const LOBBY_SIZE: i64 = 7;

const NOTES_MAX: usize = 2000;

pub fn is_host(user: &AuthUser) -> bool {
    user.role.has_at_least(Role::Host)
}

#[derive(Debug, Serialize)]
// Public on purpose: players need to reach their match's referee on Discord.
pub struct Referee {
    id: Uuid,
    osu_id: i64,
    username: String,
    avatar_url: String,
    discord_id: Option<String>,
    discord_username: Option<String>,
}

impl Referee {
    pub fn from_columns(cols: RefereeColumns) -> Option<Self> {
        match (
            cols.referee_id,
            cols.referee_osu_id,
            cols.referee_username,
            cols.referee_avatar_url,
        ) {
            (Some(id), Some(osu_id), Some(username), Some(avatar_url)) => Some(Referee {
                id,
                osu_id,
                username,
                avatar_url,
                discord_id: cols.referee_discord_id,
                discord_username: cols.referee_discord_username,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Lobby<P> {
    id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
    // Players can still join or leave: it hasn't started or been concluded.
    open: bool,
    referee: Option<Referee>,
    rooms: Vec<MatchRoom>,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
    teams: Vec<Team<P>>,
}

// Splits off the staff-only notes so the public view can drop them.
fn to_lobby<P>(row: MatchRow, teams: Vec<Team<P>>, now: OffsetDateTime) -> (Lobby<P>, String) {
    let lobby = Lobby {
        id: row.id,
        starts_at: row.starts_at,
        open: row.concluded_at.is_none() && row.starts_at > now,
        referee: Referee::from_columns(row.referee),
        rooms: match_rooms(row.mp_ids),
        concluded_at: row.concluded_at,
        teams,
    };
    (lobby, row.referee_notes)
}

// Every lobby with its teams in signup order, plus the teams that haven't signed up.
async fn load_lobbies<P>(
    state: &AppState,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Result<(Vec<(MatchRow, Vec<Team<P>>)>, Vec<Team<P>>), AppError> {
    let matches = db::matches::list_lobbies(&state.db).await?;
    let entries = db::matches::list_entries(&state.db).await?;
    let members = db::team::list_team_members(&state.db, None).await?;

    let signed_up: HashMap<Uuid, Uuid> = entries.iter().map(|e| (e.team_id, e.match_id)).collect();
    let mut teams = HashMap::new();
    let mut unassigned = Vec::new();
    for team in group_teams(members, to_player) {
        if signed_up.contains_key(&team.id) {
            teams.insert(team.id, team);
        } else {
            unassigned.push(team);
        }
    }
    let mut by_lobby: HashMap<Uuid, Vec<Team<P>>> = HashMap::new();
    for entry in entries {
        if let Some(team) = teams.remove(&entry.team_id) {
            by_lobby.entry(entry.match_id).or_default().push(team);
        }
    }
    let lobbies = matches
        .into_iter()
        .map(|m| {
            let teams = by_lobby.remove(&m.id).unwrap_or_default();
            (m, teams)
        })
        .collect();
    Ok((lobbies, unassigned))
}

// ---------- public and player views ----------

#[derive(Debug, Serialize)]
pub struct PublicQualifiers {
    lobbies: Vec<Lobby<Player>>,
    capacity: i64,
    // The signed-in player's team, so the page can show where it's signed up.
    my_team_id: Option<Uuid>,
}

// GET /api/public/qualifiers — every qualifier lobby and who's in it.
pub async fn list_public_lobbies(
    State(state): State<AppState>,
    user: Option<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    let now = OffsetDateTime::now_utc();
    let (lobbies, _) = load_lobbies(&state, |row| row.player).await?;
    let my_team_id = match user {
        Some(user) => db::team::find_team_id(&state.db, user.id).await?,
        None => None,
    };
    Ok(Json(PublicQualifiers {
        lobbies: lobbies
            .into_iter()
            .map(|(m, teams)| to_lobby(m, teams, now).0)
            .collect(),
        capacity: LOBBY_SIZE,
        my_team_id,
    }))
}

async fn my_team_id(state: &AppState, user: &AuthUser) -> Result<Uuid, AppError> {
    db::team::find_team_id(&state.db, user.id)
        .await?
        .ok_or(AppError::BadRequest("you need a team first"))
}

#[derive(Debug, Deserialize)]
pub struct JoinBody {
    match_id: Uuid,
}

// Signs a team up for a lobby, moving it out of its current one. Players can only
// move between lobbies that haven't started; hosts can move teams any time. Either
// way a lobby never holds more than LOBBY_SIZE teams.
async fn move_team(
    state: &AppState,
    team_id: Uuid,
    match_id: Uuid,
    by_host: bool,
) -> Result<StatusCode, AppError> {
    let now = OffsetDateTime::now_utc();
    let mut tx = state.db.begin().await?;
    let target = db::matches::lock_lobby(&mut tx, match_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !by_host && !target.open(now) {
        return Err(AppError::Conflict("that lobby is closed"));
    }
    if let Some((current, lobby)) = db::matches::lock_team_lobby(&mut tx, team_id).await? {
        if current == match_id {
            return Ok(StatusCode::NO_CONTENT);
        }
        if !by_host && !lobby.open(now) {
            return Err(AppError::Conflict("your lobby has already started"));
        }
    }
    if db::matches::count_entries(&mut tx, match_id).await? >= LOBBY_SIZE {
        return Err(AppError::Conflict("that lobby is full"));
    }
    match db::matches::set_entry(&mut tx, team_id, match_id).await {
        Ok(()) => {}
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("qualifier_entries_team_id_fkey") =>
        {
            return Err(AppError::NotFound)
        }
        Err(err) => return Err(err.into()),
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// PUT /api/me/qualifier — either player signs their team up for a lobby, or moves
// it to another one.
pub async fn join_lobby(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<JoinBody>,
) -> Result<impl IntoResponse, AppError> {
    let team_id = my_team_id(&state, &user).await?;
    move_team(&state, team_id, body.match_id, false).await
}

// DELETE /api/me/qualifier — either player takes their team out of its lobby
// before it starts.
pub async fn leave_lobby(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let team_id = my_team_id(&state, &user).await?;
    let mut tx = state.db.begin().await?;
    let (_, lobby) = db::matches::lock_team_lobby(&mut tx, team_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !lobby.open(OffsetDateTime::now_utc()) {
        return Err(AppError::Conflict("your lobby has already started"));
    }
    db::matches::delete_entry(&mut *tx, team_id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- staff view and lobby management ----------

#[derive(Debug, Serialize)]
pub struct StaffLobby {
    #[serde(flatten)]
    lobby: Lobby<AdminTeamPlayer>,
    referee_notes: String,
}

#[derive(Debug, Serialize)]
pub struct StaffQualifiers {
    lobbies: Vec<StaffLobby>,
    capacity: i64,
    // Teams that haven't picked a lobby yet.
    unassigned: Vec<Team<AdminTeamPlayer>>,
}

// GET /api/qualifiers — staff. Like the public list, plus referee notes, each
// player's Discord and the teams without a lobby.
pub async fn list_lobbies(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let now = OffsetDateTime::now_utc();
    let (lobbies, unassigned) = load_lobbies(&state, AdminTeamPlayer::from_row).await?;
    Ok(Json(StaffQualifiers {
        lobbies: lobbies
            .into_iter()
            .map(|(m, teams)| {
                let (lobby, referee_notes) = to_lobby(m, teams, now);
                StaffLobby {
                    lobby,
                    referee_notes,
                }
            })
            .collect(),
        capacity: LOBBY_SIZE,
        unassigned,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ScheduleBody {
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
}

// POST /api/qualifiers — host-only. Opens a new lobby at the given time.
pub async fn create_lobby(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ScheduleBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::matches::create_lobby(&state.db, body.starts_at).await?;
    Ok(StatusCode::NO_CONTENT)
}

// PATCH /api/qualifiers/:id — host-only. Moves a lobby to another time.
pub async fn reschedule_lobby(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ScheduleBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::matches::reschedule_lobby(&state.db, id, body.starts_at).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/qualifiers/:id — host-only. Its teams go back to having no lobby.
pub async fn delete_lobby(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::matches::delete_lobby(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// PUT /api/teams/:id/qualifier — host-only. Puts a team in a lobby, even one that
// has started or been concluded.
pub async fn assign_team(
    State(state): State<AppState>,
    user: AuthUser,
    Path(team_id): Path<Uuid>,
    Json(body): Json<JoinBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    move_team(&state, team_id, body.match_id, true).await
}

// DELETE /api/teams/:id/qualifier — host-only. Takes a team out of its lobby.
pub async fn unassign_team(
    State(state): State<AppState>,
    user: AuthUser,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::matches::delete_entry(&state.db, team_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// GET /api/referees — host-only. Everyone a match can be given to.
pub async fn list_referees(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    Ok(Json(db::matches::list_referees(&state.db).await?))
}

// ---------- running a match ----------

pub async fn load_match(state: &AppState, id: Uuid) -> Result<MatchState, AppError> {
    db::matches::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)
}

// A match is run by its referee or by a host; other referees can only look.
pub fn require_runs(user: &AuthUser, m: &MatchState) -> Result<(), AppError> {
    user.require_role(Role::Referee)?;
    if is_host(user) || m.referee_id == Some(user.id) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

// Once a match is concluded only hosts can change it (notes aside).
pub fn require_unconcluded(user: &AuthUser, m: &MatchState) -> Result<(), AppError> {
    if m.concluded() && !is_host(user) {
        return Err(AppError::Conflict("this match is concluded"));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RefereeBody {
    referee_id: Option<Uuid>,
}

// PUT /api/matches/:id/referee — a referee takes a free match (their own id) or
// gives up theirs (null). Hosts can give any match to any staff member, or clear it.
pub async fn set_referee(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RefereeBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let m = load_match(&state, id).await?;

    if is_host(&user) {
        if let Some(referee_id) = body.referee_id {
            if !db::matches::can_referee(&state.db, referee_id).await? {
                return Err(AppError::BadRequest("only staff can referee"));
            }
        }
        db::matches::set_referee(&state.db, id, body.referee_id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    require_unconcluded(&user, &m)?;
    match body.referee_id {
        Some(referee_id) if referee_id == user.id => {
            if m.referee_id != Some(user.id)
                && !db::matches::claim_match(&state.db, id, user.id).await?
            {
                return Err(AppError::Conflict("this match already has a referee"));
            }
        }
        None => {
            if !db::matches::release_match(&state.db, id, user.id).await? {
                return Err(AppError::Forbidden);
            }
        }
        Some(_) => return Err(AppError::Forbidden),
    }
    Ok(StatusCode::NO_CONTENT)
}

// One of a match's osu! multiplayer rooms. Usually a match has one; a room that
// closes or breaks mid-match is replaced, and the old one stays listed.
#[derive(Debug, Serialize)]
pub struct MatchRoom {
    mp_id: i64,
    url: String,
}

pub fn match_rooms(mp_ids: Vec<i64>) -> Vec<MatchRoom> {
    mp_ids
        .into_iter()
        .map(|mp_id| MatchRoom {
            mp_id,
            url: format!("https://osu.ppy.sh/community/matches/{mp_id}"),
        })
        .collect()
}

// Accepts a multiplayer room link (osu.ppy.sh/community/matches/<id> or
// osu.ppy.sh/mp/<id>, with or without the scheme) or the bare room id.
fn parse_mp_id(raw: &str) -> Option<i64> {
    let s = raw.trim();
    let s = s.split(['?', '#']).next().unwrap_or_default();
    let s = s.trim_end_matches('/');
    let id = if s.bytes().all(|b| b.is_ascii_digit()) {
        s
    } else {
        let path = s
            .strip_prefix("https://")
            .or_else(|| s.strip_prefix("http://"))
            .unwrap_or(s)
            .strip_prefix("osu.ppy.sh/")?;
        path.strip_prefix("community/matches/")
            .or_else(|| path.strip_prefix("mp/"))?
    };
    id.parse().ok().filter(|&id| id > 0)
}

#[derive(Debug, Deserialize)]
pub struct RoomBody {
    link: String,
}

// POST /api/matches/:id/rooms — the match's referee adds a room (its link or id)
// until the match is concluded; hosts can any time. A room belongs to one match.
pub async fn add_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RoomBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    require_runs(&user, &m)?;
    require_unconcluded(&user, &m)?;
    let mp_id = parse_mp_id(&body.link)
        .ok_or(AppError::BadRequest("that isn't an osu! multiplayer link"))?;
    match db::matches::add_room(&state.db, id, mp_id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("match_rooms_pkey") => {
            Err(AppError::Conflict("that room is already linked to a match"))
        }
        Err(err) => Err(err.into()),
    }
}

// DELETE /api/matches/:id/rooms/:mp_id — takes a mistaken room off the match, on
// the same terms as adding one. A concluded match keeps at least one. The room's
// saved chat stays in the database.
pub async fn remove_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, mp_id)): Path<(Uuid, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    require_runs(&user, &m)?;
    require_unconcluded(&user, &m)?;
    if m.concluded() && m.mp_ids == [mp_id] {
        return Err(AppError::BadRequest(
            "a concluded match needs a room, reopen it first",
        ));
    }
    if !db::matches::remove_room(&state.db, id, mp_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct NotesBody {
    notes: String,
}

// PUT /api/matches/:id/notes — the match's referee or a host notes down anything
// unusual, also after the match is concluded. Staff-only.
pub async fn set_notes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<NotesBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    require_runs(&user, &m)?;
    if body.notes.chars().count() > NOTES_MAX {
        return Err(AppError::BadRequest("notes can be at most 2000 characters"));
    }
    db::matches::set_notes(&state.db, id, &body.notes).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/matches/:id/conclude — the match's referee or a host marks it as
// played. Needs a room; a bracket match also needs its score, and concluding
// it sends its teams on to their next matches.
pub async fn conclude(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    require_runs(&user, &m)?;
    if m.concluded() {
        return Err(AppError::Conflict("this match is already concluded"));
    }
    if m.mp_ids.is_empty() {
        return Err(AppError::BadRequest("add the match's room first"));
    }
    if m.kind == MatchKind::Bracket {
        bracket::conclude(&state, id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    if !db::matches::conclude(&state.db, id).await? {
        return Err(AppError::Conflict(
            "this match changed, reload and try again",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/matches/:id/reopen — host-only. Undoes a conclusion, taking a bracket
// match's teams back out of the matches they advanced to.
pub async fn reopen(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let m = load_match(&state, id).await?;
    if m.kind == MatchKind::Bracket {
        bracket::reopen(&state, id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    db::matches::reopen(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
