use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, StatusCode},
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
        team::{InviteRow, Player, PlayerState, TeamMemberRow},
    },
    discord::jobs::Job,
    error::AppError,
    rank_refresh,
    role::{Role, Roles},
    team_rank, AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/players/search", get(search_players))
        .route(
            "/me/team",
            get(my_team_status)
                .patch(rename_my_team)
                .delete(disband_my_team),
        )
        .route(
            "/me/team/banner",
            put(upload_my_banner)
                .delete(remove_my_banner)
                .layer(DefaultBodyLimit::max(BANNER_MAX_BYTES)),
        )
        .route("/invites", post(send_invite))
        .route("/invites/:id", delete(delete_invite))
        .route("/invites/:id/accept", post(accept_invite))
        .route("/public/teams", get(list_public_teams))
        .route("/public/teams/:id/banner", get(team_banner))
        .route("/teams", get(list_teams))
        .route("/teams/:id", patch(rename_team).delete(disband_team))
        .route("/teams/:id/banner", delete(remove_team_banner))
        .route("/ranks/refresh", post(refresh_ranks))
        .route("/settings/teams-frozen", put(set_teams_frozen))
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
        if !Roles::of(&other.roles).can_play() {
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

// Set on a formed team whose ranks no longer fit. Hosts decide what happens next.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RankWarning {
    OverLimit,
    Unranked,
}

#[derive(Debug, Serialize)]
pub struct Team<P> {
    pub id: Uuid,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    // Versioned by upload time, so the image can be cached forever.
    banner_url: Option<String>,
    team_value: Option<f64>,
    rank_warning: Option<RankWarning>,
    players: Vec<P>,
}

#[derive(Debug, Serialize)]
pub struct AdminTeamPlayer {
    #[serde(flatten)]
    player: Player,
    // For copying a mention (<@id>) that pings them on Discord.
    discord_id: Option<String>,
    discord_username: Option<String>,
    // False once they've left the tournament's Discord server.
    discord_in_guild: Option<bool>,
}

impl AdminTeamPlayer {
    pub fn from_row(row: TeamMemberRow) -> Self {
        Self {
            player: row.player,
            discord_id: row.discord_id,
            discord_username: row.discord_username,
            discord_in_guild: row.discord_in_guild,
        }
    }
}

// Rows arrive ordered by team, so consecutive rows with the same team_id belong together.
pub fn group_teams<P>(
    rows: Vec<TeamMemberRow>,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Vec<Team<P>> {
    let mut teams: Vec<(Team<P>, Vec<Player>)> = Vec::new();
    for row in rows {
        if teams.last().map(|(t, _)| t.id) != Some(row.team_id) {
            let team = Team {
                id: row.team_id,
                name: row.team_name.clone(),
                created_at: row.team_created_at,
                banner_url: row.banner_updated_at.map(|at| {
                    let version = at.unix_timestamp_nanos() / 1_000_000;
                    format!("/api/public/teams/{}/banner?v={version}", row.team_id)
                }),
                team_value: None,
                rank_warning: None,
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
                team.team_value = a
                    .global_rank
                    .zip(b.global_rank)
                    .map(|(a, b)| team_rank::team_value(a, b));
            }
            team.rank_warning = match team.team_value {
                None => Some(RankWarning::Unranked),
                Some(value) if value > team_rank::LIMIT => Some(RankWarning::OverLimit),
                Some(_) => None,
            };
            team
        })
        .collect()
}

// Only basic users and streamers play; the rest of the staff run the tournament.
// Forming a team also needs a linked Discord account.
fn require_player(user: &AuthUser) -> Result<(), AppError> {
    if !user.roles.can_play() {
        return Err(AppError::BadRequest("staff can't join teams"));
    }
    user.require_discord_verified()
}

// Players can't change anything about teams once hosts freeze them. Host actions
// skip this check.
async fn require_unfrozen(ex: impl sqlx::PgExecutor<'_>) -> Result<(), AppError> {
    if db::settings::teams_frozen(ex).await? {
        return Err(AppError::Conflict("teams are frozen"));
    }
    Ok(())
}

async fn load_self(state: &AppState, user: &AuthUser) -> Result<PlayerState, AppError> {
    db::team::find_player(&state.db, user.id)
        .await?
        .ok_or(AppError::Unauthenticated)
}

const TEAM_NAME_MAX: usize = 32;

fn parse_team_name(raw: &str) -> Result<String, AppError> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || name.chars().count() > TEAM_NAME_MAX {
        return Err(AppError::BadRequest(
            "team names must be 1 to 32 characters",
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "team names can't contain control characters",
        ));
    }
    Ok(name)
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
    let mut rows = db::team::search_players(&state.db, user.id, query).await?;
    let ids: Vec<Uuid> = rows
        .iter()
        .map(|row| row.state.player.id)
        .chain([user.id])
        .collect();
    if rank_refresh::refresh_stale(&state, &ids).await > 0 {
        rows = db::team::search_players(&state.db, user.id, query).await?;
    }
    let me = load_self(&state, &user).await?.player;
    let results = rows
        .into_iter()
        .map(|row| PlayerResult {
            eligibility: Eligibility::of(&me, &row.state),
            team_value: me
                .global_rank
                .zip(row.state.player.global_rank)
                .map(|(a, b)| team_rank::team_value(a, b)),
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
    team_name: String,
    player: Player,
    eligibility: Eligibility,
    team_value: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TeamStatus {
    team: Option<Team<Player>>,
    frozen: bool,
    global_rank: Option<i64>,
    min_partner_rank: Option<i64>,
    limit: f64,
    sent: Vec<InviteView>,
    received: Vec<InviteView>,
    // The players of the caller's team who left the tournament's Discord server.
    left_discord: Vec<Uuid>,
}

// GET /api/me/team — the caller's team (if any) and their pending invites.
pub async fn my_team_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let mut members = db::team::list_team_members(&state.db, Some(user.id)).await?;
    let ids: Vec<Uuid> = members
        .iter()
        .map(|row| row.player.id)
        .chain([user.id])
        .collect();
    if rank_refresh::refresh_stale(&state, &ids).await > 0 {
        members = db::team::list_team_members(&state.db, Some(user.id)).await?;
    }
    let me = load_self(&state, &user).await?.player;
    let left_discord = members
        .iter()
        .filter(|row| row.discord_in_guild == Some(false))
        .map(|row| row.player.id)
        .collect();
    let team = group_teams(members, |row| row.player).pop();

    let view = |row: InviteRow| InviteView {
        id: row.invite_id,
        created_at: row.created_at,
        team_name: row.team_name,
        eligibility: Eligibility::of(&me, &row.state),
        team_value: me
            .global_rank
            .zip(row.state.player.global_rank)
            .map(|(a, b)| team_rank::team_value(a, b)),
        player: row.state.player,
    };
    let sent = db::team::list_sent_invites(&state.db, user.id).await?;
    let received = db::team::list_received_invites(&state.db, user.id).await?;

    Ok(Json(TeamStatus {
        team,
        frozen: db::settings::teams_frozen(&state.db).await?,
        global_rank: me.global_rank,
        min_partner_rank: me.global_rank.map(team_rank::min_partner_rank),
        limit: team_rank::LIMIT,
        sent: sent.into_iter().map(view).collect(),
        received: received.into_iter().map(view).collect(),
        left_discord,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SendInviteBody {
    user_id: Uuid,
    team_name: String,
}

// POST /api/invites — invite another player to team up under the given team name.
pub async fn send_invite(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SendInviteBody>,
) -> Result<impl IntoResponse, AppError> {
    require_player(&user)?;
    if body.user_id == user.id {
        return Err(AppError::BadRequest("you can't invite yourself"));
    }
    let team_name = parse_team_name(&body.team_name)?;
    require_unfrozen(&state.db).await?;
    require_in_server(&state, user.id).await?;
    rank_refresh::refresh_stale(&state, &[user.id, body.user_id]).await;
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
    db::team::create_invite(&state.db, user.id, body.user_id, &team_name)
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
    let inviter_id = db::team::received_invite_inviter(&state.db, id, user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_in_server(&state, user.id).await?;
    rank_refresh::refresh_stale(&state, &[user.id, inviter_id]).await;

    let mut tx = state.db.begin().await?;
    require_unfrozen(&mut *tx).await?;
    let (inviter_id, team_name) = db::team::lock_received_invite(&mut tx, id, user.id)
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

    let team_id = match db::team::create_team(&mut tx, inviter_id, user.id, &team_name).await {
        Ok(team_id) => team_id,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("teams_name_key") => {
            return Err(AppError::Conflict("that team name is taken"))
        }
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("team_members_pkey") => {
            return Err(AppError::Conflict("one of you is already in a team"))
        }
        Err(err) => return Err(err.into()),
    };
    state
        .discord
        .enqueue(
            &mut *tx,
            &[
                Job::SyncUser {
                    user_id: inviter_id,
                },
                Job::SyncUser { user_id: user.id },
                Job::TeamFormed { team_id },
            ],
        )
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// While the bot runs, a player who left the tournament's Discord server has to come
// back before forming a team: their matches are arranged there.
async fn require_in_server(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    if !state.discord.enabled() {
        return Ok(());
    }
    match db::discord::account_of(&state.db, user_id).await? {
        Some((_, Some(false))) => Err(AppError::Conflict(
            "rejoin the tournament's Discord server first",
        )),
        _ => Ok(()),
    }
}

#[derive(Debug, Deserialize)]
pub struct RenameBody {
    name: String,
}

// Match channels are named after their teams, so they're renamed too.
async fn rename(state: &AppState, team_id: Uuid, raw_name: &str) -> Result<StatusCode, AppError> {
    let name = parse_team_name(raw_name)?;
    let mut tx = state.db.begin().await?;
    match db::team::rename_team(&mut *tx, team_id, &name).await {
        Ok(true) => {}
        Ok(false) => return Err(AppError::NotFound),
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("teams_name_key") => {
            return Err(AppError::Conflict("that team name is taken"))
        }
        Err(err) => return Err(err.into()),
    }
    let jobs: Vec<Job> = db::discord::open_matches_of_team(&mut *tx, team_id)
        .await?
        .into_iter()
        .map(|match_id| Job::SyncMatch { match_id })
        .collect();
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// Deletes a team. Its players lose the Player role and its matches' channels lose them.
async fn disband(state: &AppState, team_id: Uuid, by_host: bool) -> Result<bool, AppError> {
    let mut tx = state.db.begin().await?;
    let players = db::discord::team_players(&mut *tx, team_id).await?;
    let matches = db::discord::open_matches_of_team(&mut *tx, team_id).await?;
    if !db::team::delete_team(&mut *tx, team_id).await? {
        return Ok(false);
    }
    let mut jobs: Vec<Job> = players
        .iter()
        .map(|p| Job::SyncUser { user_id: p.user_id })
        .collect();
    jobs.extend(
        matches
            .into_iter()
            .map(|match_id| Job::SyncMatch { match_id }),
    );
    if let Some(first) = players.first() {
        jobs.push(Job::TeamDisbanded {
            name: first.team_name.clone(),
            players: players.iter().map(|p| p.user_id).collect(),
            by_host,
        });
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(true)
}

async fn my_team_id(state: &AppState, user: &AuthUser) -> Result<Uuid, AppError> {
    db::team::find_team_id(&state.db, user.id)
        .await?
        .ok_or(AppError::NotFound)
}

// PATCH /api/me/team — either player renames their team.
pub async fn rename_my_team(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<RenameBody>,
) -> Result<impl IntoResponse, AppError> {
    require_unfrozen(&state.db).await?;
    let team_id = my_team_id(&state, &user).await?;
    rename(&state, team_id, &body.name).await
}

// DELETE /api/me/team — either player disbands their team.
pub async fn disband_my_team(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    require_unfrozen(&state.db).await?;
    let team_id = my_team_id(&state, &user).await?;
    disband(&state, team_id, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- banners ----------

const BANNER_MAX_BYTES: usize = 2 * 1024 * 1024;

// Trusts the file's magic bytes rather than the request's Content-Type.
fn banner_content_type(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

// PUT /api/me/team/banner — either player sets their team's banner. The body is
// the raw image.
pub async fn upload_my_banner(
    State(state): State<AppState>,
    user: AuthUser,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    require_unfrozen(&state.db).await?;
    let team_id = my_team_id(&state, &user).await?;
    let content_type = banner_content_type(&body).ok_or(AppError::BadRequest(
        "banners must be PNG, JPEG, GIF or WebP images",
    ))?;
    db::team::set_banner(&state.db, team_id, content_type, &body).await?;
    sync_team_matches(&state, team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// The team's open matches show its banner on their channel cards.
async fn sync_team_matches(state: &AppState, team_id: Uuid) -> Result<(), AppError> {
    let jobs: Vec<Job> = db::discord::open_matches_of_team(&state.db, team_id)
        .await?
        .into_iter()
        .map(|match_id| Job::SyncMatch { match_id })
        .collect();
    state.discord.enqueue(&state.db, &jobs).await?;
    Ok(())
}

// DELETE /api/me/team/banner
pub async fn remove_my_banner(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    require_unfrozen(&state.db).await?;
    let team_id = my_team_id(&state, &user).await?;
    db::team::delete_banner(&state.db, team_id).await?;
    sync_team_matches(&state, team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// GET /api/public/teams/:id/banner
pub async fn team_banner(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let banner = db::team::find_banner(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let headers = [
        (header::CONTENT_TYPE, banner.content_type),
        (
            header::CACHE_CONTROL,
            "public, max-age=31536000, immutable".to_string(),
        ),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
    ];
    Ok((headers, banner.data))
}

// ---------- team lists ----------

#[derive(Debug, Serialize)]
pub struct TeamList<P> {
    teams: Vec<Team<P>>,
    frozen: bool,
    limit: f64,
    // When every team member's rank was last refreshed.
    #[serde(with = "time::serde::rfc3339::option")]
    ranks_updated_at: Option<OffsetDateTime>,
}

async fn team_list<P>(
    state: &AppState,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Result<TeamList<P>, AppError> {
    let rows = db::team::list_team_members(&state.db, None).await?;
    let mut teams = group_teams(rows, to_player);
    // Highest team value first, unranked teams last; ties stay in name order.
    teams.sort_by(|a, b| {
        let value = |t: &Team<P>| t.team_value.unwrap_or(f64::NEG_INFINITY);
        value(b).total_cmp(&value(a))
    });
    Ok(TeamList {
        teams,
        frozen: db::settings::teams_frozen(&state.db).await?,
        limit: team_rank::LIMIT,
        ranks_updated_at: db::settings::ranks_updated_at(&state.db).await?,
    })
}

// GET /api/public/teams — every team with its players.
pub async fn list_public_teams(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(team_list(&state, |row| row.player).await?))
}

// GET /api/teams — host-only. Like the public list, plus each player's Discord.
pub async fn list_teams(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let list = team_list(&state, AdminTeamPlayer::from_row).await?;
    Ok(Json(list))
}

// PATCH /api/teams/:id — host-only. Renames a team.
pub async fn rename_team(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RenameBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    rename(&state, id, &body.name).await
}

// DELETE /api/teams/:id — host-only. Disbands a team; its players become free agents.
pub async fn disband_team(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !disband(&state, id, true).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/teams/:id/banner — host-only.
pub async fn remove_team_banner(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::team::delete_banner(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    sync_team_matches(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct RefreshResult {
    updated: u64,
    #[serde(with = "time::serde::rfc3339")]
    ranks_updated_at: OffsetDateTime,
}

// POST /api/ranks/refresh — host-only. Runs the daily rank refresh right away.
pub async fn refresh_ranks(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let refresh = rank_refresh::refresh_team_members(&state).await?;
    Ok(Json(RefreshResult {
        updated: refresh.updated,
        ranks_updated_at: refresh.at,
    }))
}

#[derive(Debug, Deserialize)]
pub struct FreezeBody {
    frozen: bool,
}

// PUT /api/settings/teams-frozen — host-only. Freezes or unfreezes every team.
pub async fn set_teams_frozen(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<FreezeBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::settings::set_teams_frozen(&state.db, body.frozen).await?;
    Ok(StatusCode::NO_CONTENT)
}
