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
        bracket::{BanRow, ManualResultRow, MapResultRow, PickRow, Side},
        chat::ChatMessage,
        mappool::{Category, PublicPoolMap, PublicSlot},
        matches::{MatchKind, RoomRow},
        team::{Player, TeamMemberRow},
    },
    error::AppError,
    handlers::{
        bracket::Streamer,
        matches::{match_rooms, MatchRoom, Referee},
        team::{group_teams, AdminTeamPlayer, Team},
    },
    osu_api,
    role::Role,
    AppState,
};

// Routes under the `/api` nest. The referee page reads a match from here and runs
// the osu! room over IRC (handlers/irc.rs); changes to the match itself still go
// through the shared `/api/matches/:id/*` routes. The public match page reads the
// same match, minus what's staff-only, from `/api/public/matches/:id`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/matches/:id/room", get(get_room))
        .route("/matches/:id/chat", get(get_chat))
        .route("/matches/:id/osu-games", get(get_osu_games))
        .route("/public/matches/:id", get(get_public_match))
        .route("/public/matches/:id/chat", get(get_public_chat))
}

#[derive(Debug, Serialize)]
pub struct RoomStage {
    id: Uuid,
    name: String,
    published: bool,
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
    // Oldest first; the last one is where the match is being played.
    rooms: Vec<MatchRoom>,
    referee_notes: String,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
    // Bracket matches only: which of `teams` plays in which slot (red, blue).
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    // Bracket matches only: the slot of the team that forfeited, once decided that way.
    forfeit_slot: Option<i16>,
    // A bracket match's teams, or a lobby's in signup order.
    teams: Vec<Team<AdminTeamPlayer>>,
    // The map pool it's played in, published or not; no editor notes.
    stage: Option<RoomStage>,
    categories: Vec<Category>,
    slots: Vec<PublicSlot>,
    maps: Vec<PublicPoolMap>,
    // Bracket matches only: the pool maps each team banned and picked, in the order
    // they were made.
    bans: Vec<BanRow>,
    picks: Vec<PickRow>,
    // Bracket matches only: map wins the referee awarded by hand, oldest first.
    manual_results: Vec<ManualResultRow>,
    // Bracket matches only: the pool maps the saved score was counted from, as the
    // referee page last saved them.
    map_results: Vec<MapResultRow>,
    // The match has a Discord channel, so clicking a player's Discord name pings them
    // there (handlers/discord.rs).
    discord_channel: bool,
}

// A match's teams as its room plays them: a bracket match's two (red, blue), or a
// lobby's in signup order.
async fn room_teams<P>(
    state: &AppState,
    row: &RoomRow,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Result<Vec<Team<P>>, AppError> {
    let members = db::team::list_team_members(&state.db, None).await?;
    let mut all = group_teams(members, to_player);
    let order: Vec<Uuid> = match row.kind {
        MatchKind::Bracket => [row.team1_id, row.team2_id].into_iter().flatten().collect(),
        MatchKind::Qualifier => db::matches::list_entries(&state.db)
            .await?
            .into_iter()
            .filter(|e| e.match_id == row.id)
            .map(|e| e.team_id)
            .collect(),
    };
    Ok(order
        .into_iter()
        .filter_map(|team_id| {
            let i = all.iter().position(|t| t.id == team_id)?;
            Some(all.swap_remove(i))
        })
        .collect())
}

// The map pool a match is played in.
struct RoomPool {
    categories: Vec<Category>,
    slots: Vec<PublicSlot>,
    maps: Vec<PublicPoolMap>,
}

impl RoomPool {
    const EMPTY: RoomPool = RoomPool {
        categories: Vec::new(),
        slots: Vec::new(),
        maps: Vec::new(),
    };

    async fn load(state: &AppState, stage_id: Uuid) -> Result<Self, AppError> {
        Ok(RoomPool {
            categories: db::mappool::list_categories(&state.db, stage_id).await?,
            slots: db::mappool::list_public_slots(&state.db, stage_id).await?,
            maps: db::mappool::list_public_maps(&state.db, stage_id).await?,
        })
    }
}

// What a bracket match's referee has marked and saved; nothing for a lobby.
struct RoomMarks {
    bans: Vec<BanRow>,
    picks: Vec<PickRow>,
    manual_results: Vec<ManualResultRow>,
    map_results: Vec<MapResultRow>,
}

impl RoomMarks {
    async fn load(state: &AppState, row: &RoomRow) -> Result<Self, AppError> {
        Ok(match row.kind {
            MatchKind::Bracket => RoomMarks {
                bans: db::bracket::list_bans(&state.db, row.id).await?,
                picks: db::bracket::list_picks(&state.db, row.id).await?,
                manual_results: db::bracket::list_manual_results(&state.db, row.id).await?,
                map_results: db::bracket::list_map_results(&state.db, row.id).await?,
            },
            MatchKind::Qualifier => RoomMarks {
                bans: Vec::new(),
                picks: Vec::new(),
                manual_results: Vec::new(),
                map_results: Vec::new(),
            },
        })
    }
}

fn room_stage(row: &RoomRow) -> Option<RoomStage> {
    match (row.stage_id, &row.stage_name) {
        (Some(id), Some(name)) => Some(RoomStage {
            id,
            name: name.clone(),
            published: row.stage_published.unwrap_or(false),
        }),
        _ => None,
    }
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
    let teams = room_teams(&state, &row, AdminTeamPlayer::from_row).await?;
    let stage = room_stage(&row);
    let pool = match &stage {
        Some(stage) => RoomPool::load(&state, stage.id).await?,
        None => RoomPool::EMPTY,
    };
    let marks = RoomMarks::load(&state, &row).await?;

    Ok(Json(RefereeRoom {
        id: row.id,
        kind: row.kind,
        side: row.side,
        number: row.number,
        lobby_number: row.lobby_number,
        starts_at: row.starts_at,
        referee: Referee::from_columns(row.referee),
        rooms: match_rooms(row.mp_ids),
        referee_notes: row.referee_notes,
        concluded_at: row.concluded_at,
        team1_id: row.team1_id,
        team2_id: row.team2_id,
        team1_score: row.team1_score,
        team2_score: row.team2_score,
        forfeit_slot: row.forfeit_slot,
        teams,
        stage,
        categories: pool.categories,
        slots: pool.slots,
        maps: pool.maps,
        bans: marks.bans,
        picks: marks.picks,
        manual_results: marks.manual_results,
        map_results: marks.map_results,
        discord_channel: state.discord.enabled()
            && db::discord::channel_of(&state.db, id).await?.is_some(),
    }))
}

// A match as its public page shows it: the referee page's view without the
// referee's notes and the players' Discords, and with the pool only once its stage
// is published.
#[derive(Debug, Serialize)]
pub struct PublicMatch {
    id: Uuid,
    kind: MatchKind,
    side: Option<Side>,
    number: Option<i32>,
    lobby_number: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    // Staff Discord accounts are public on purpose.
    referee: Option<Referee>,
    streamer: Option<Streamer>,
    rooms: Vec<MatchRoom>,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    forfeit_slot: Option<i16>,
    teams: Vec<Team<Player>>,
    stage: Option<RoomStage>,
    categories: Vec<Category>,
    slots: Vec<PublicSlot>,
    maps: Vec<PublicPoolMap>,
    bans: Vec<BanRow>,
    picks: Vec<PickRow>,
    manual_results: Vec<ManualResultRow>,
    map_results: Vec<MapResultRow>,
}

// GET /api/public/matches/:id — anyone. A match for its public page, which follows
// it live (handlers/live.rs).
pub async fn get_public_match(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = db::matches::find_room(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let teams = room_teams(&state, &row, |r| r.player).await?;
    let stage = room_stage(&row);
    // An unpublished pool stays hidden, even while its matches are played.
    let pool = match &stage {
        Some(stage) if stage.published => RoomPool::load(&state, stage.id).await?,
        _ => RoomPool::EMPTY,
    };
    let marks = RoomMarks::load(&state, &row).await?;

    Ok(Json(PublicMatch {
        id: row.id,
        kind: row.kind,
        side: row.side,
        number: row.number,
        lobby_number: row.lobby_number,
        starts_at: row.starts_at,
        referee: Referee::from_columns(row.referee),
        streamer: Streamer::from_columns(
            row.streamer_id,
            row.streamer_osu_id,
            row.streamer_username,
            row.streamer_avatar_url,
        ),
        rooms: match_rooms(row.mp_ids),
        concluded_at: row.concluded_at,
        team1_id: row.team1_id,
        team2_id: row.team2_id,
        team1_score: row.team1_score,
        team2_score: row.team2_score,
        forfeit_slot: row.forfeit_slot,
        teams,
        stage,
        categories: pool.categories,
        slots: pool.slots,
        maps: pool.maps,
        bans: marks.bans,
        picks: marks.picks,
        manual_results: marks.manual_results,
        map_results: marks.map_results,
    }))
}

#[derive(Debug, Serialize)]
pub struct RoomChat {
    mp_id: i64,
    messages: Vec<ChatMessage>,
}

// What was said in each of the match's rooms while a referee's relay was in it,
// saved as it happened: rooms oldest first, each room's lines oldest first.
async fn load_chat(state: &AppState, id: Uuid) -> Result<Vec<RoomChat>, AppError> {
    let m = db::matches::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut rooms = Vec::with_capacity(m.mp_ids.len());
    for mp_id in m.mp_ids {
        let messages = db::chat::list_messages(&state.db, mp_id).await?;
        rooms.push(RoomChat { mp_id, messages });
    }
    Ok(rooms)
}

// GET /api/matches/:id/chat — referees and up. The saved chat of the match's rooms.
pub async fn get_chat(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    Ok(Json(load_chat(&state, id).await?))
}

// GET /api/public/matches/:id/chat — anyone. The same chat for the match page: only
// the match's own rooms, which everyone in them could read anyway.
pub async fn get_public_chat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(load_chat(&state, id).await?))
}

#[derive(Debug, Serialize)]
pub struct OsuGameScore {
    osu_id: i64,
    score: i64,
    passed: bool,
    // The mods the player played with, as acronyms ("HD", "HR").
    mods: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OsuGame {
    beatmap_id: Option<i64>,
    // The room's mods, as acronyms.
    mods: Vec<String>,
    scores: Vec<OsuGameScore>,
}

#[derive(Debug, Serialize)]
pub struct OsuRoomGames {
    mp_id: i64,
    // Every finished map of the room in the order played, or null when osu! has no
    // record of it (a mistyped or test link).
    games: Option<Vec<OsuGame>>,
}

// GET /api/matches/:id/osu-games — referees and up. Each of the match's rooms as osu!
// recorded it, for when the room's chat missed something (the relay was down, nobody
// had the page open): the referee page can count the score from these instead.
pub async fn get_osu_games(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let m = db::matches::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    let mut rooms = Vec::with_capacity(m.mp_ids.len());
    for mp_id in m.mp_ids {
        let games = match osu_api::matches::fetch_games(&state.http_client, &token, mp_id).await {
            Ok(games) => Some(games),
            Err(AppError::OsuApiRequest(err)) if osu_api::matches::refused(&err) => {
                tracing::warn!(error = %err, mp_id, match_id = %id, "osu! has no readable record of the room");
                None
            }
            Err(err) => return Err(err),
        };
        rooms.push(OsuRoomGames {
            mp_id,
            games: games.map(|games| {
                games
                    .into_iter()
                    .map(|g| OsuGame {
                        beatmap_id: g.beatmap_id,
                        mods: g.mods,
                        scores: g
                            .scores
                            .into_iter()
                            .map(|s| OsuGameScore {
                                osu_id: s.user_id,
                                score: s.score,
                                passed: s.passed,
                                mods: s.mods,
                            })
                            .collect(),
                    })
                    .collect()
            }),
        });
    }
    Ok(Json(rooms))
}
