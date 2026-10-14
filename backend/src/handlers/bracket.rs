use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{
        self,
        bracket::{BracketLock, BracketRow, LinkRow, MatchUpdate, Outcome, Side},
        matches::MatchKind,
        notifications::StaffRole,
        team::{Player, TeamMemberRow},
    },
    discord::jobs::Job,
    error::AppError,
    handlers::{
        matches::{
            is_host, load_match, match_rooms, require_runs, require_unconcluded, MatchRoom, Referee,
        },
        team::{group_teams, AdminTeamPlayer, Team},
    },
    role::Role,
    AppState,
};

// Routes under the `/api` nest. Hosts build the bracket; referees run its matches
// through the shared `/api/matches/:id/*` routes, and streamers sign up to stream them.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/bracket", get(list_public_bracket))
        .route("/bracket", get(list_bracket).post(create_match))
        .route("/bracket/:id", put(update_match).delete(delete_match))
        .route("/bracket/:id/position", put(move_match))
        .route("/bracket/:id/score", put(set_score))
        .route("/bracket/:id/bans/:map_id", put(set_ban).delete(remove_ban))
        .route(
            "/bracket/:id/picks/:map_id",
            put(set_pick).delete(remove_pick),
        )
        .route("/bracket/:id/streamer", put(set_streamer))
        .route("/streamers", get(list_streamers))
}

const SCORE_MAX: i32 = 99;

#[derive(Debug, Serialize)]
pub struct Streamer {
    id: Uuid,
    osu_id: i64,
    username: String,
    avatar_url: String,
}

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
    published: bool,
    position: i32,
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
    winner_to: Option<NextMatch>,
    loser_to: Option<NextMatch>,
    stage: Option<MatchStage>,
    referee: Option<Referee>,
    streamer: Option<Streamer>,
    rooms: Vec<MatchRoom>,
    #[serde(with = "time::serde::rfc3339::option")]
    concluded_at: Option<OffsetDateTime>,
}

// Splits off the staff-only notes so the public view can drop them.
fn to_match(row: BracketRow, links: &[LinkRow]) -> (BracketMatch, String) {
    let next = |outcome| {
        links
            .iter()
            .find(|l| l.source_id == row.id && l.outcome == outcome)
            .map(|l| NextMatch {
                match_id: l.target_id,
                slot: l.slot,
            })
    };
    let streamer = match (
        row.streamer_id,
        row.streamer_osu_id,
        row.streamer_username,
        row.streamer_avatar_url,
    ) {
        (Some(id), Some(osu_id), Some(username), Some(avatar_url)) => Some(Streamer {
            id,
            osu_id,
            username,
            avatar_url,
        }),
        _ => None,
    };
    let stage = match (
        row.stage_id,
        row.stage_name,
        row.stage_published,
        row.stage_position,
    ) {
        (Some(id), Some(name), Some(published), Some(position)) => Some(MatchStage {
            id,
            name,
            published,
            position,
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
        winner_to: next(Outcome::Winner),
        loser_to: next(Outcome::Loser),
        stage,
        referee: Referee::from_columns(row.referee),
        streamer,
        rooms: match_rooms(row.mp_ids),
        concluded_at: row.concluded_at,
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
    let matches = rows.into_iter().map(|row| to_match(row, &links)).collect();
    Ok((matches, group_teams(members, to_player)))
}

#[derive(Debug, Serialize)]
pub struct PublicBracket {
    matches: Vec<BracketMatch>,
    teams: Vec<Team<Player>>,
    // The signed-in player's team, so the page can point out their matches.
    my_team_id: Option<Uuid>,
}

// GET /api/public/bracket — every bracket match.
pub async fn list_public_bracket(
    State(state): State<AppState>,
    user: Option<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    let (matches, teams) = load_bracket(&state, |row| row.player).await?;
    let my_team_id = match user {
        Some(user) => db::team::find_team_id(&state.db, user.id).await?,
        None => None,
    };
    Ok(Json(PublicBracket {
        matches: matches.into_iter().map(|(m, _)| m).collect(),
        teams,
        my_team_id,
    }))
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

// GET /api/bracket — referees and up. Like the public list, plus referee notes and
// each player's Discord. Streamers can play, so they get the public view instead.
pub async fn list_bracket(
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
            Some("bracket_matches_code_key") => {
                return AppError::Conflict("another match already has that number")
            }
            Some("bracket_links_target_key") => {
                return AppError::Conflict("that slot already gets a team from another match")
            }
            Some("bracket_links_target_id_fkey") => return AppError::NotFound,
            Some("bracket_matches_team1_id_fkey" | "bracket_matches_team2_id_fkey") => {
                return AppError::BadRequest("that team doesn't exist")
            }
            Some("bracket_matches_stage_id_fkey") => {
                return AppError::BadRequest("that stage doesn't exist")
            }
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
pub async fn create_match(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let id = db::bracket::create_match(&state.db, body.side, body.pos_x, body.pos_y)
        .await
        .map_err(edit_error)?;
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
// of the match loses its proposed times for it.
pub async fn update_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MatchBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if body.number < 1 {
        return Err(AppError::BadRequest("match numbers start at 1"));
    }
    if body.team1_id.is_some() && body.team1_id == body.team2_id {
        return Err(AppError::BadRequest("a team can't play itself"));
    }
    let mut new_links = Vec::new();
    for (outcome, next) in [
        (Outcome::Winner, body.winner_to),
        (Outcome::Loser, body.loser_to),
    ] {
        let Some(next) = next else { continue };
        if next.slot != 1 && next.slot != 2 {
            return Err(AppError::BadRequest("a match has only two slots"));
        }
        if next.match_id == id {
            return Err(AppError::BadRequest("a match can't lead into itself"));
        }
        new_links.push(LinkRow {
            source_id: id,
            outcome,
            target_id: next.match_id,
            slot: next.slot,
        });
    }

    let mut tx = state.db.begin().await?;
    let current = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound)?;
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
        return Err(AppError::Conflict(
            "this match is concluded, reopen it before changing its teams or links",
        ));
    }
    let targets: Vec<Uuid> = new_links.iter().map(|l| l.target_id).collect();
    if leads_back(&links, id, &targets) {
        return Err(AppError::BadRequest(
            "that link would make the bracket loop back to this match",
        ));
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
    // gets its channel now has the time in its welcome.
    let mut jobs = vec![Job::SyncMatch { match_id: id }];
    if current.starts_at != body.starts_at && db::discord::channel_of(&mut *tx, id).await?.is_some()
    {
        jobs.push(Job::TimeSet { match_id: id });
    }
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await.map_err(edit_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MoveBody {
    pos_x: i32,
    pos_y: i32,
}

// PUT /api/bracket/:id/position — host-only. Moves a match on the editor's board.
// The editor keeps cards from overlapping; the server doesn't need to.
pub async fn move_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<MoveBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::bracket::move_match(&state.db, id, body.pos_x, body.pos_y).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id — host-only. Teams it already sent on stay where they are.
pub async fn delete_match(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let channel = db::discord::channel_of(&state.db, id).await?;
    if !db::bracket::delete_match(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    if let Some(channel_id) = channel {
        state
            .discord
            .enqueue(
                &state.db,
                &[Job::ArchiveChannel {
                    channel_id,
                    kind: MatchKind::Bracket,
                }],
            )
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ScoreBody {
    team1_score: Option<i32>,
    team2_score: Option<i32>,
}

// PUT /api/bracket/:id/score — the match's referee or a host enters the score.
// The score decides who moves on, so a concluded match has to be reopened first.
pub async fn set_score(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ScoreBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(AppError::NotFound);
    }
    require_runs(&user, &m)?;
    let valid = |score: Option<i32>| score.is_none_or(|s| (0..=SCORE_MAX).contains(&s));
    if !valid(body.team1_score) || !valid(body.team2_score) {
        return Err(AppError::BadRequest("scores must be between 0 and 99"));
    }
    if !db::bracket::set_score(&state.db, id, body.team1_score, body.team2_score).await? {
        return Err(AppError::Conflict(
            "this match is concluded, reopen it to change the score",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MapTeamBody {
    team_id: Uuid,
}

// Bans and picks are marked by the match's referee or a host until the match is
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
        return Err(AppError::NotFound);
    }
    require_runs(user, &m)?;
    require_unconcluded(user, &m)?;
    let teams = db::bracket::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if Some(team_id) != teams.team1_id && Some(team_id) != teams.team2_id {
        return Err(AppError::BadRequest("that team doesn't play in this match"));
    }
    if !db::bracket::map_in_pool(&state.db, id, map_id).await? {
        return Err(AppError::BadRequest("that map isn't in this match's pool"));
    }
    Ok(())
}

// Taking a ban or pick back: the same people, on the same terms.
async fn check_unmark(state: &AppState, user: &AuthUser, id: Uuid) -> Result<(), AppError> {
    let m = load_match(state, id).await?;
    require_runs(user, &m)?;
    require_unconcluded(user, &m)
}

// PUT /api/bracket/:id/bans/:map_id — marks a map as banned by one of the match's
// teams (or moves the ban to the other team). A picked map can't be banned.
pub async fn set_ban(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, map_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<MapTeamBody>,
) -> Result<impl IntoResponse, AppError> {
    check_map_team(&state, &user, id, map_id, body.team_id).await?;
    if !db::bracket::set_ban(&state.db, id, map_id, body.team_id).await? {
        return Err(AppError::Conflict("that map has been picked"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id/bans/:map_id — takes a ban back.
pub async fn remove_ban(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, map_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_unmark(&state, &user, id).await?;
    if !db::bracket::delete_ban(&state.db, id, map_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// PUT /api/bracket/:id/picks/:map_id — marks a map as picked by one of the match's
// teams (or moves the pick to the other team). A banned map can't be picked.
pub async fn set_pick(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, map_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<MapTeamBody>,
) -> Result<impl IntoResponse, AppError> {
    check_map_team(&state, &user, id, map_id, body.team_id).await?;
    if !db::bracket::set_pick(&state.db, id, map_id, body.team_id).await? {
        return Err(AppError::Conflict("that map is banned"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id/picks/:map_id — takes a pick back.
pub async fn remove_pick(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, map_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_unmark(&state, &user, id).await?;
    if !db::bracket::delete_pick(&state.db, id, map_id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// The winner and loser of a match with both teams and a score that isn't a tie.
fn outcome(m: &BracketLock) -> Option<(Uuid, Uuid)> {
    let (t1, t2) = (m.team1_id?, m.team2_id?);
    match m.team1_score?.cmp(&m.team2_score?) {
        std::cmp::Ordering::Greater => Some((t1, t2)),
        std::cmp::Ordering::Less => Some((t2, t1)),
        std::cmp::Ordering::Equal => None,
    }
}

// Concludes a bracket match and puts its winner and loser into the slots they're
// linked to. Called by matches::conclude once it has checked who's asking.
pub async fn conclude(state: &AppState, id: Uuid) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    let m = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if m.concluded() {
        return Err(AppError::Conflict("this match is already concluded"));
    }
    if !m.has_room {
        return Err(AppError::BadRequest("add the match's room first"));
    }
    if m.team1_id.is_none() || m.team2_id.is_none() {
        return Err(AppError::BadRequest("this match needs both teams first"));
    }
    let (winner, loser) =
        outcome(&m).ok_or(AppError::BadRequest("enter a score with a winner first"))?;
    // The channel is archived, the results posted, and the matches the teams move on to
    // may now have both teams and so their channel.
    let mut jobs = vec![
        Job::SyncMatch { match_id: id },
        Job::Concluded { match_id: id },
        Job::PostResults { match_id: id },
    ];
    for target in db::bracket::lock_targets(&mut tx, id).await? {
        if target.concluded_at.is_some() {
            return Err(AppError::Conflict(
                "a match this one leads into is already concluded, reopen it first",
            ));
        }
        let team = match target.outcome {
            Outcome::Winner => winner,
            Outcome::Loser => loser,
        };
        db::bracket::set_slot(&mut tx, target.target_id, target.slot, Some(team)).await?;
        jobs.push(Job::SyncMatch {
            match_id: target.target_id,
        });
    }
    db::reschedule::delete_for_match(&mut tx, id).await?;
    db::matches::conclude(&mut *tx, id).await?;
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}

// Reopens a bracket match and takes its teams back out of the slots it put them in,
// unless a host has put another team there since.
pub async fn reopen(state: &AppState, id: Uuid) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    let m = db::bracket::lock_match(&mut tx, id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !m.concluded() {
        return Ok(());
    }
    let result = outcome(&m);
    let mut jobs = vec![
        Job::SyncMatch { match_id: id },
        Job::Reopened { match_id: id },
        Job::DeleteResults { match_id: id },
    ];
    for target in db::bracket::lock_targets(&mut tx, id).await? {
        if target.concluded_at.is_some() {
            return Err(AppError::Conflict(
                "a match this one leads into is already concluded, reopen it first",
            ));
        }
        let Some((winner, loser)) = result else {
            continue;
        };
        let team = match target.outcome {
            Outcome::Winner => winner,
            Outcome::Loser => loser,
        };
        let current = if target.slot == 1 {
            target.team1_id
        } else {
            target.team2_id
        };
        if current == Some(team) {
            db::bracket::set_slot(&mut tx, target.target_id, target.slot, None).await?;
            jobs.push(Job::SyncMatch {
                match_id: target.target_id,
            });
        }
    }
    db::matches::reopen(&mut *tx, id).await?;
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct StreamerBody {
    streamer_id: Option<Uuid>,
}

// PUT /api/bracket/:id/streamer — a streamer takes a free match (their own id) or
// gives up theirs (null), but can't stream a match their own team plays in. Hosts
// can give any match to any streamer, or clear it.
pub async fn set_streamer(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<StreamerBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Streamer)?;
    let m = db::bracket::find_match(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    if is_host(&user) {
        if let Some(streamer_id) = body.streamer_id {
            if !db::bracket::can_stream(&state.db, streamer_id).await? {
                return Err(AppError::BadRequest("only staff can stream"));
            }
        }
        db::bracket::set_streamer(&state.db, id, body.streamer_id).await?;
        streamer_changed(&state, &user, id, m.streamer_id, body.streamer_id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    if m.concluded() {
        return Err(AppError::Conflict("this match is concluded"));
    }
    match body.streamer_id {
        Some(streamer_id) if streamer_id == user.id => {
            if m.streamer_id == Some(user.id) {
                return Ok(StatusCode::NO_CONTENT);
            }
            if let Some(team_id) = db::team::find_team_id(&state.db, user.id).await? {
                if m.team1_id == Some(team_id) || m.team2_id == Some(team_id) {
                    return Err(AppError::BadRequest("you can't stream a match you play in"));
                }
            }
            if !db::bracket::claim_stream(&state.db, id, user.id).await? {
                return Err(AppError::Conflict("this match already has a streamer"));
            }
        }
        None => {
            if !db::bracket::release_stream(&state.db, id, user.id).await? {
                return Err(AppError::Forbidden);
            }
        }
        Some(_) => return Err(AppError::Forbidden),
    }
    streamer_changed(&state, &user, id, m.streamer_id, body.streamer_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// The streamer joins (or leaves) the match's channel, which says so.
async fn streamer_changed(
    state: &AppState,
    user: &AuthUser,
    id: Uuid,
    before: Option<Uuid>,
    after: Option<Uuid>,
) -> Result<(), AppError> {
    if before == after {
        return Ok(());
    }
    state
        .discord
        .enqueue(
            &state.db,
            &[
                Job::SyncMatch { match_id: id },
                Job::StaffChanged {
                    match_id: id,
                    role: StaffRole::Streamer,
                    user_id: after,
                    by: user.id,
                },
            ],
        )
        .await?;
    Ok(())
}

// GET /api/streamers — host-only. Everyone a match's stream can be given to.
pub async fn list_streamers(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    Ok(Json(db::bracket::list_streamers(&state.db).await?))
}
