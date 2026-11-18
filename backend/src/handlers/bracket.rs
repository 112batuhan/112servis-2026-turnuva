use std::collections::{HashMap, HashSet};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{is_host, require_runs, require_unconcluded};
use crate::{
    auth::AuthUser,
    db::{
        self,
        bracket::{BracketRow, LinkRow, MapResultRow, MatchUpdate, Outcome, Side},
        matches::MatchKind,
        notifications::StaffRole,
        potential::{Before, PotentialRow},
        team::{Player, TeamMemberRow},
    },
    discord::jobs::Job,
    error::{AppError, AuthError, BracketError, GeneralError, MatchError, PoolError, StaffError},
    role::Role,
    services::{
        self,
        bracket::{conclude, deletion_jobs, save_score, Mark},
        matches::load_match,
    },
    stats_cache,
    views::{group_teams, match_rooms, AdminTeamPlayer, MatchRoom, Referee, Streamer, Team},
    AppState,
};

// Routes under the `/api` nest. Hosts build the bracket; referees run its matches
// through the shared `/api/matches/:id/*` routes, and streamers sign up to stream them.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/bracket", get(list_public_bracket))
        .route(
            "/bracket",
            get(list_bracket).post(create_match).delete(remove_bracket),
        )
        .route("/bracket/{id}", put(update_match).delete(delete_match))
        .route("/bracket/{id}/position", put(move_match))
        .route("/bracket/{id}/score", put(set_score))
        .route("/bracket/{id}/forfeit", post(forfeit))
        .route("/bracket/{id}/results", post(add_manual_result))
        .route(
            "/bracket/{id}/results/{result_id}",
            delete(remove_manual_result),
        )
        .route(
            "/bracket/{id}/bans/{map_id}",
            put(set_ban).delete(remove_ban),
        )
        .route(
            "/bracket/{id}/picks/{map_id}",
            put(set_pick).delete(remove_pick),
        )
        .route("/bracket/{id}/missed/{missed_id}", delete(remove_missed))
        .route("/bracket/{id}/streamer", put(set_streamer))
        .route("/streamers", get(list_streamers))
}

const RESULT_NOTE_MAX: usize = 200;
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NextMatch {
    match_id: Uuid,
    // 1 or 2: which team slot of that match the team goes into.
    slot: i16,
}

// The stage a match is played in. Its name is public either way; `published` says
// whether its map pool is on the public page yet, and `position` orders the stages.
#[derive(Debug, Serialize)]
pub struct MatchStage {
    id: Uuid,
    name: String,
    // Its short name for the board's cards ("QF"); null when the hosts haven't given one.
    abbreviation: Option<String>,
    published: bool,
    position: i32,
    // How many maps the match is played to; null when the hosts haven't set it.
    best_of: Option<i32>,
}

// A pairing an undecided bracket match may turn out to be (crate::potentials): its
// teams can agree on a time and its staff sign up, but it has no room, score or page of
// its own until it's decided.
#[derive(Debug, Serialize)]
pub struct Potential {
    id: Uuid,
    team1_id: Uuid,
    team2_id: Uuid,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    referee: Option<Referee>,
    streamer: Option<Streamer>,
}

impl Potential {
    fn from_row(row: PotentialRow) -> Self {
        Potential {
            id: row.id,
            team1_id: row.team1_id,
            team2_id: row.team2_id,
            starts_at: row.starts_at,
            referee: Referee::from_columns(row.referee),
            streamer: Streamer::from_columns(
                row.streamer_id,
                row.streamer_osu_id,
                row.streamer_username,
                row.streamer_avatar_url,
            ),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BracketMatch {
    id: Uuid,
    side: Side,
    // Numbered within its side: W1, W2, ... and L1, L2, ...
    number: i32,
    // Top left corner on the editor's board, in board units. The board has no edges,
    // so either can be negative.
    pos_x: i32,
    pos_y: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    // 1 or 2: the team that forfeited, when the match was decided that way. The other
    // one won, whatever the score says.
    forfeit_slot: Option<i16>,
    winner_to: Option<NextMatch>,
    loser_to: Option<NextMatch>,
    stage: Option<MatchStage>,
    referee: Option<Referee>,
    streamer: Option<Streamer>,
    rooms: Vec<MatchRoom>,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
    // The pairings it may still turn out to be, soonest first; empty once it's decided
    // (or when nothing of its week decides it).
    potentials: Vec<Potential>,
}

// Splits off the staff-only notes so the public view can drop them.
fn to_match(
    row: BracketRow,
    links: &[LinkRow],
    potentials: &mut HashMap<Uuid, Vec<Potential>>,
) -> (BracketMatch, String) {
    let next = |outcome| {
        links
            .iter()
            .find(|l| l.source_id == row.id && l.outcome == outcome)
            .map(|l| NextMatch {
                match_id: l.target_id,
                slot: l.slot,
            })
    };
    let streamer = Streamer::from_columns(
        row.streamer_id,
        row.streamer_osu_id,
        row.streamer_username,
        row.streamer_avatar_url,
    );
    let stage = match (
        row.stage_id,
        row.stage_name,
        row.stage_published,
        row.stage_position,
    ) {
        (Some(id), Some(name), Some(published), Some(position)) => Some(MatchStage {
            id,
            name,
            abbreviation: row.stage_abbreviation,
            published,
            position,
            best_of: row.stage_best_of,
        }),
        _ => None,
    };
    let m = BracketMatch {
        id: row.id,
        side: row.side,
        number: row.number,
        pos_x: row.pos_x,
        pos_y: row.pos_y,
        starts_at: row.starts_at,
        team1_id: row.team1_id,
        team2_id: row.team2_id,
        team1_score: row.team1_score,
        team2_score: row.team2_score,
        forfeit_slot: row.forfeit_slot,
        winner_to: next(Outcome::Winner),
        loser_to: next(Outcome::Loser),
        stage,
        referee: Referee::from_columns(row.referee),
        streamer,
        rooms: match_rooms(row.mp_ids),
        concluded_at: row.concluded_at,
        potentials: potentials.remove(&row.id).unwrap_or_default(),
    };
    (m, row.referee_notes)
}

// Every bracket match, plus every team so the matches can refer to them by id.
async fn load_bracket<P>(
    state: &AppState,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Result<(Vec<(BracketMatch, String)>, Vec<Team<P>>), AppError> {
    let rows = db::bracket::list_matches(&state.db).await?;
    let links = db::bracket::list_links(&state.db).await?;
    let members = db::team::list_team_members(&state.db, None).await?;
    let mut potentials: HashMap<Uuid, Vec<Potential>> = HashMap::new();
    for row in db::potential::list(&state.db).await? {
        potentials
            .entry(row.bracket_match_id)
            .or_default()
            .push(Potential::from_row(row));
    }
    let matches = rows
        .into_iter()
        .map(|row| to_match(row, &links, &mut potentials))
        .collect();
    Ok((matches, group_teams(members, to_player)))
}

// The same for everyone, so it's kept between changes (crate::stats_cache); the
// response adds `my_team_id`, the signed-in player's team, so the page can point out
// their matches.
#[derive(Debug, Serialize)]
pub struct PublicBracket {
    matches: Vec<BracketMatch>,
    teams: Vec<Team<Player>>,
}

// GET /api/public/bracket — every bracket match.
async fn list_public_bracket(
    State(state): State<AppState>,
    user: Option<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    let shared = state
        .bracket_cache
        .get_or_count(Uuid::nil(), || async {
            let (matches, teams) = load_bracket(&state, |row| row.player).await?;
            let bracket = PublicBracket {
                matches: matches.into_iter().map(|(m, _)| m).collect(),
                teams,
            };
            Ok(Bytes::from(serde_json::to_vec(&bracket)?))
        })
        .await?;
    let my_team_id = match user {
        Some(user) => db::team::find_team_id(&state.db, user.id).await?,
        None => None,
    };
    let body = stats_cache::with_field(&shared, "my_team_id", &my_team_id)?;
    Ok(([(header::CONTENT_TYPE, "application/json")], body))
}

#[derive(Debug, Serialize)]
pub struct StaffBracketMatch {
    #[serde(flatten)]
    m: BracketMatch,
    referee_notes: String,
}

#[derive(Debug, Serialize)]
pub struct StaffBracket {
    matches: Vec<StaffBracketMatch>,
    teams: Vec<Team<AdminTeamPlayer>>,
}

// GET /api/bracket — referees and hosts. Like the public list, plus referee notes and
// each player's Discord. Other staff (streamers can play) get the public view instead.
async fn list_bracket(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    let (matches, teams) = load_bracket(&state, AdminTeamPlayer::from_row).await?;
    Ok(Json(StaffBracket {
        matches: matches
            .into_iter()
            .map(|(m, referee_notes)| StaffBracketMatch { m, referee_notes })
            .collect(),
        teams,
    }))
}

// Turns the constraint errors an edit can run into into messages for the editor.
fn edit_error(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(e) = &err {
        match e.constraint() {
            Some("bracket_matches_code_key") => return BracketError::MatchNumberTaken.into(),
            Some("bracket_links_target_key") => return BracketError::SlotHasSource.into(),
            Some("bracket_links_target_id_fkey") => return GeneralError::NotFound.into(),
            Some("bracket_matches_team1_id_fkey" | "bracket_matches_team2_id_fkey") => {
                return BracketError::UnknownTeam.into()
            }
            Some("bracket_matches_stage_id_fkey") => return PoolError::UnknownStage.into(),
            _ => {}
        }
    }
    err.into()
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    side: Side,
    pos_x: i32,
    pos_y: i32,
}

#[derive(Debug, Serialize)]
pub struct Created {
    id: Uuid,
}

// POST /api/bracket — host-only. Adds an empty match at a board position, numbered
// after the last match on its side.
async fn create_match(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let id = db::bracket::create_match(&state.db, body.side, body.pos_x, body.pos_y)
        .await
        .map_err(edit_error)?;
    state.live.bracket_changed();
    Ok(Json(Created { id }))
}

#[derive(Debug, Deserialize)]
pub struct MatchBody {
    side: Side,
    number: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    winner_to: Option<NextMatch>,
    loser_to: Option<NextMatch>,
    stage_id: Option<Uuid>,
}

// Whether following the links out of `targets` leads back to `id`, with `id`'s own
// links left out of `links`.
fn leads_back(links: &[LinkRow], id: Uuid, targets: &[Uuid]) -> bool {
    let mut next: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for link in links.iter().filter(|l| l.source_id != id) {
        next.entry(link.source_id).or_default().push(link.target_id);
    }
    let mut stack = targets.to_vec();
    let mut seen = HashSet::new();
    while let Some(m) = stack.pop() {
        if m == id {
            return true;
        }
        if seen.insert(m) {
            stack.extend(next.get(&m).into_iter().flatten());
        }
    }
    false
}

// PUT /api/bracket/:id — host-only. Saves everything the match editor edits. A
// concluded match's teams and links are what moved its teams on, so those need the
// match reopened first; its time, code and stage can still change. A team taken out
// of the match loses its proposed times for it. The potential matches are worked out
// again, and the ones still on the match's old time follow it to the new one.
async fn update_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MatchBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if body.number < 1 {
        return Err(BracketError::BadMatchNumber.into());
    }
    if body.team1_id.is_some() && body.team1_id == body.team2_id {
        return Err(BracketError::TeamPlaysItself.into());
    }
    let mut new_links = Vec::new();
    for (outcome, next) in [
        (Outcome::Winner, body.winner_to),
        (Outcome::Loser, body.loser_to),
    ] {
        let Some(next) = next else { continue };
        if next.slot != 1 && next.slot != 2 {
            return Err(BracketError::BadSlot.into());
        }
        if next.match_id == id {
            return Err(BracketError::LinkToItself.into());
        }
        new_links.push(LinkRow {
            source_id: id,
            outcome,
            target_id: next.match_id,
            slot: next.slot,
        });
    }

    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let current = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    let links = db::bracket::list_links(&mut *tx).await?;
    let mut old_links: Vec<LinkRow> = links
        .iter()
        .filter(|l| l.source_id == id)
        .copied()
        .collect();
    old_links.sort_by_key(|l| l.outcome == Outcome::Loser);

    if current.concluded()
        && (current.team1_id != body.team1_id
            || current.team2_id != body.team2_id
            || old_links != new_links)
    {
        return Err(MatchError::ConcludedTeams.into());
    }
    let targets: Vec<Uuid> = new_links.iter().map(|l| l.target_id).collect();
    if leads_back(&links, id, &targets) {
        return Err(BracketError::Loop.into());
    }

    let update = MatchUpdate {
        side: body.side,
        number: body.number,
        starts_at: body.starts_at,
        team1_id: body.team1_id,
        team2_id: body.team2_id,
        stage_id: body.stage_id,
    };
    db::bracket::update_match(&mut tx, id, &update)
        .await
        .map_err(edit_error)?;
    if current.team1_id != body.team1_id || current.team2_id != body.team2_id {
        db::reschedule::delete_stale(&mut tx, id).await?;
    }
    if old_links != new_links {
        db::bracket::set_links(&mut tx, id, &new_links)
            .await
            .map_err(edit_error)?;
    }
    // A match that already has a channel hears about its new time there; one that
    // gets its channel now has the time on its card.
    let mut jobs = vec![Job::SyncMatch { match_id: id }];
    let mut notified = Vec::new();
    if current.starts_at != body.starts_at {
        for match_id in
            db::potential::follow_time(&mut tx, id, current.starts_at, body.starts_at).await?
        {
            jobs.extend([Job::SyncMatch { match_id }, Job::TimeSet { match_id }]);
        }
    }
    let before = Before {
        match_id: id,
        team1_id: current.team1_id,
        team2_id: current.team2_id,
    };
    jobs.extend(db::potential::sync(&mut tx, &[before]).await?);
    // Taking over a pairing brings its agreed time; the one the host just set wins.
    if current.starts_at != body.starts_at {
        db::reschedule::set_time(&mut tx, id, body.starts_at).await?;
    }
    if current.starts_at != body.starts_at && db::discord::channel_of(&mut *tx, id).await?.is_some()
    {
        jobs.push(Job::TimeSet { match_id: id });
    }
    // Its referee and streamer are asked whether they can make the new time, as when
    // the teams move it. A time cleared or set in the past (a record) asks nobody.
    if let Some(new_starts_at) = body.starts_at {
        if current.starts_at != body.starts_at
            && !current.concluded()
            && new_starts_at > OffsetDateTime::now_utc()
            && (current.referee_id.is_some() || current.streamer_id.is_some())
        {
            notified = db::notifications::ask_confirm_time(
                &mut tx,
                id,
                user.id,
                current.referee_id,
                current.streamer_id,
                current.starts_at,
                new_starts_at,
            )
            .await?;
            jobs.push(Job::ConfirmTime { match_id: id });
        }
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await.map_err(edit_error)?;
    state.live.bracket_changed();
    state.live.notifications_changed(notified);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MoveBody {
    pos_x: i32,
    pos_y: i32,
}

// PUT /api/bracket/:id/position — host-only. Moves a match on the editor's board.
// The editor keeps cards from overlapping; the server doesn't need to.
async fn move_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::bracket::move_match(&state.db, id, body.pos_x, body.pos_y).await? {
        return Err(GeneralError::NotFound.into());
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket — host-only. Takes the whole bracket down while none of it has
// been played, e.g. to make it again from a corrected seeding.
async fn remove_bracket(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    services::seeding::remove_bracket(&state).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id — host-only. Teams it already sent on stay where they are.
// Its potential matches go with it, and the ones it decided are worked out again.
async fn delete_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let mut jobs = db::potential::remove_of_match(&mut tx, id).await?;
    jobs.extend(deletion_jobs(&mut tx, id).await?);
    // Its rooms go with it; the bot closes the ones it made (crate::bancho).
    if !db::bracket::delete_match(&mut *tx, id).await? {
        return Err(GeneralError::NotFound.into());
    }
    jobs.extend(db::potential::sync(&mut tx, &[]).await?);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ScoreBody {
    team1_score: Option<i32>,
    team2_score: Option<i32>,
    // The pool maps the score was counted from, which replace the saved ones (the
    // results post lists them). Left out, they stay as they are.
    #[serde(default)]
    maps: Option<Vec<MapResultRow>>,
}

// PUT /api/bracket/:id/score — a referee or a host enters the score by hand (the bots
// save the one they count from the rooms themselves, see crate::bancho). The score
// decides who moves on, so a concluded match has to be reopened first.
async fn set_score(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ScoreBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(GeneralError::NotFound.into());
    }
    require_runs(&user)?;
    save_score(
        &state,
        id,
        body.team1_score,
        body.team2_score,
        body.maps.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MapTeamBody {
    team_id: Uuid,
}

// Bans and picks are marked by a referee or a host until the match is
// concluded (hosts any time), for one of its two teams and a map of its pool.
async fn check_map_team(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    map_id: Uuid,
    team_id: Uuid,
) -> Result<(), AppError> {
    let m = load_match(state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(GeneralError::NotFound.into());
    }
    require_runs(user)?;
    require_unconcluded(user, &m)?;
    let teams = db::bracket::find_match(&state.db, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    if Some(team_id) != teams.team1_id && Some(team_id) != teams.team2_id {
        return Err(MatchError::TeamNotInMatch.into());
    }
    if !db::bracket::map_in_pool(&state.db, id, map_id).await? {
        return Err(MatchError::MapNotInPool.into());
    }
    Ok(())
}

// Taking a ban or pick back: the same people, on the same terms.
async fn check_unmark(state: &AppState, user: &AuthUser, id: Uuid) -> Result<(), AppError> {
    let m = load_match(state, id).await?;
    require_runs(user)?;
    require_unconcluded(user, &m)
}

// Marks a map as banned or picked by one of the match's teams (or moves the mark to the
// other team).
async fn set_mark(
    state: &AppState,
    user: &AuthUser,
    (id, map_id): (Uuid, Uuid),
    team_id: Uuid,
    mark: Mark,
) -> Result<StatusCode, AppError> {
    check_map_team(state, user, id, map_id, team_id).await?;
    services::bracket::mark(state, id, map_id, team_id, mark).await?;
    Ok(StatusCode::NO_CONTENT)
}

// Takes a ban or a pick back.
async fn remove_mark(
    state: &AppState,
    user: &AuthUser,
    (id, map_id): (Uuid, Uuid),
    mark: Mark,
) -> Result<StatusCode, AppError> {
    check_unmark(state, user, id).await?;
    let removed = match mark {
        Mark::Ban => db::bracket::delete_ban(&state.db, id, map_id).await?,
        Mark::Pick => db::bracket::delete_pick(&state.db, id, map_id).await?,
    };
    if !removed {
        return Err(GeneralError::NotFound.into());
    }
    services::bracket::marks_changed(state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// PUT /api/bracket/:id/bans/:map_id — marks a map as banned by one of the match's
// teams (or moves the ban to the other team). A picked map can't be banned.
async fn set_ban(
    State(state): State<AppState>,
    user: AuthUser,
    Path(ids): Path<(Uuid, Uuid)>,
    Json(body): Json<MapTeamBody>,
) -> Result<impl IntoResponse, AppError> {
    set_mark(&state, &user, ids, body.team_id, Mark::Ban).await
}

// DELETE /api/bracket/:id/bans/:map_id — takes a ban back.
async fn remove_ban(
    State(state): State<AppState>,
    user: AuthUser,
    Path(ids): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    remove_mark(&state, &user, ids, Mark::Ban).await
}

// DELETE /api/bracket/:id/missed/:missed_id — takes back a ban or pick the auto referee
// recorded as run out, so the team gets its turn again.
async fn remove_missed(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, missed_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_unmark(&state, &user, id).await?;
    services::bracket::take_back_miss(&state, id, missed_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// PUT /api/bracket/:id/picks/:map_id — marks a map as picked by one of the match's
// teams (or moves the pick to the other team). A banned map can't be picked.
async fn set_pick(
    State(state): State<AppState>,
    user: AuthUser,
    Path(ids): Path<(Uuid, Uuid)>,
    Json(body): Json<MapTeamBody>,
) -> Result<impl IntoResponse, AppError> {
    set_mark(&state, &user, ids, body.team_id, Mark::Pick).await
}

// DELETE /api/bracket/:id/picks/:map_id — takes a pick back.
async fn remove_pick(
    State(state): State<AppState>,
    user: AuthUser,
    Path(ids): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    remove_mark(&state, &user, ids, Mark::Pick).await
}

#[derive(Debug, Deserialize)]
pub struct ForfeitBody {
    // The slot (1 or 2) of the team that forfeits.
    slot: i16,
}

// POST /api/bracket/:id/forfeit — a referee or a host concludes it by
// forfeit: a team didn't show up or the match couldn't be finished. The other team
// moves on as the winner. Needs both teams, but no room or score.
async fn forfeit(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ForfeitBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(GeneralError::NotFound.into());
    }
    require_runs(&user)?;
    if !(1..=2).contains(&body.slot) {
        return Err(BracketError::BadSlot.into());
    }
    conclude(&state, id, Some(body.slot)).await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ManualResultBody {
    team_id: Uuid,
    // A map of the match's pool, or none for a point given without one.
    map_id: Option<Uuid>,
    #[serde(default)]
    note: String,
}

// Hand-awarded map wins change the score, so like the score they're for a referee or
// a host, and fixed once it's concluded.
async fn check_results(state: &AppState, user: &AuthUser, id: Uuid) -> Result<(), AppError> {
    let m = load_match(state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(GeneralError::NotFound.into());
    }
    require_runs(user)?;
    if m.concluded() {
        return Err(MatchError::ConcludedResults.into());
    }
    Ok(())
}

// POST /api/bracket/:id/results — awards a map (or a point without one) to one of the
// match's teams by hand, for a map the room didn't settle. The referee page counts it
// with the maps read from the room.
async fn add_manual_result(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ManualResultBody>,
) -> Result<impl IntoResponse, AppError> {
    check_results(&state, &user, id).await?;
    let teams = db::bracket::find_match(&state.db, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    if Some(body.team_id) != teams.team1_id && Some(body.team_id) != teams.team2_id {
        return Err(MatchError::TeamNotInMatch.into());
    }
    if let Some(map_id) = body.map_id {
        if !db::bracket::map_in_pool(&state.db, id, map_id).await? {
            return Err(MatchError::MapNotInPool.into());
        }
    }
    let note = body.note.trim();
    if note.chars().count() > RESULT_NOTE_MAX {
        return Err(MatchError::NoteTooLong.into());
    }
    let result_id =
        db::bracket::add_manual_result(&state.db, id, body.map_id, body.team_id, note).await?;
    state.live.match_changed(id);
    Ok((StatusCode::CREATED, Json(Created { id: result_id })))
}

// DELETE /api/bracket/:id/results/:result_id — takes a hand-awarded map win back.
async fn remove_manual_result(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, result_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_results(&state, &user, id).await?;
    if !db::bracket::delete_manual_result(&state.db, id, result_id).await? {
        return Err(GeneralError::NotFound.into());
    }
    state.live.match_changed(id);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct StreamerBody {
    streamer_id: Option<Uuid>,
}

// PUT /api/bracket/:id/streamer — a streamer takes a free match (their own id) or
// gives up theirs (null), but can't stream a match their own team plays in. Hosts
// can give any match to any streamer, or clear it. A potential match is streamed the
// same way, and a bracket match's potential matches still on its old streamer follow
// it to the new one.
async fn set_streamer(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<StreamerBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Streamer)?;
    let (m, potential) = match db::bracket::find_match(&state.db, id).await? {
        Some(m) => (m, false),
        None => (
            db::potential::find(&state.db, id)
                .await?
                .ok_or(GeneralError::NotFound)?,
            true,
        ),
    };

    if is_host(&user) {
        if let Some(streamer_id) = body.streamer_id {
            if !db::bracket::can_stream(&state.db, streamer_id).await? {
                return Err(StaffError::NotAStreamer.into());
            }
        }
        let mut tx = state.db.begin().await?;
        db::matches::set_streamer(&mut *tx, id, body.streamer_id).await?;
        streamer_changed(
            &state,
            &mut tx,
            &user,
            id,
            potential,
            m.streamer_id,
            body.streamer_id,
        )
        .await?;
        tx.commit().await?;
        state.live.bracket_changed();
        return Ok(StatusCode::NO_CONTENT);
    }

    if m.concluded() {
        return Err(MatchError::Concluded.into());
    }
    let mut tx = state.db.begin().await?;
    match body.streamer_id {
        Some(streamer_id) if streamer_id == user.id => {
            if m.streamer_id == Some(user.id) {
                state.live.bracket_changed();
                return Ok(StatusCode::NO_CONTENT);
            }
            if !user.roles.has(Role::Streamer) {
                return Err(AuthError::Forbidden.into());
            }
            if let Some(team_id) = db::team::find_team_id(&state.db, user.id).await? {
                if m.team1_id == Some(team_id) || m.team2_id == Some(team_id) {
                    return Err(StaffError::StreamOwnMatch.into());
                }
            }
            if !db::matches::claim_stream(&mut *tx, id, user.id).await? {
                return Err(StaffError::HasStreamer.into());
            }
        }
        None => {
            if !db::matches::release_stream(&mut *tx, id, user.id).await? {
                return Err(AuthError::Forbidden.into());
            }
        }
        Some(_) => return Err(AuthError::Forbidden.into()),
    }
    streamer_changed(
        &state,
        &mut tx,
        &user,
        id,
        potential,
        m.streamer_id,
        body.streamer_id,
    )
    .await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

// The streamer joins (or leaves) the match's channel, which says so, and so do the
// channels of the potential matches that follow a bracket match's streamer.
async fn streamer_changed(
    state: &AppState,
    conn: &mut PgConnection,
    user: &AuthUser,
    id: Uuid,
    potential: bool,
    before: Option<Uuid>,
    after: Option<Uuid>,
) -> Result<(), AppError> {
    if before == after {
        return Ok(());
    }
    let mut changed = vec![id];
    if !potential {
        changed.extend(db::potential::follow_streamer(&mut *conn, id, before, after).await?);
    }
    let jobs: Vec<Job> = changed
        .into_iter()
        .flat_map(|match_id| {
            [
                Job::SyncMatch { match_id },
                Job::StaffChanged {
                    match_id,
                    role: StaffRole::Streamer,
                    user_id: after,
                    by: user.id,
                },
            ]
        })
        .collect();
    state.discord.enqueue(&mut *conn, &jobs).await?;
    Ok(())
}

// GET /api/streamers — host-only. Everyone a match's stream can be given to.
async fn list_streamers(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    Ok(Json(db::bracket::list_streamers(&state.db).await?))
}
