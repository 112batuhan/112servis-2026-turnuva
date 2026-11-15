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
        pickem::{MatchRow, Mode, PredictionRow, StageRow},
        team::Player,
    },
    error::{AppError, GeneralError, MatchError, PickemError},
    pickem::{self, Status, Tally, Window},
    role::Role,
    AppState,
};

// Routes under the `/api` nest. Anyone can read the pick'em; predicting needs a linked
// Discord account, and only hosts open or lock a stage by hand.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/pickem", get(overview))
        .route("/public/pickem/matches/{id}", get(match_pickem))
        .route("/pickem/{match_id}", put(predict).delete(withdraw))
        .route("/stages/{id}/pickem", put(set_mode))
}

// Everything the pick'em is worked out from, read at once. The windows follow the
// bracket matches alone; potential matches are predicted while their stage is open.
struct Snapshot {
    stages: Vec<StageRow>,
    matches: Vec<MatchRow>,
    potentials: Vec<MatchRow>,
    windows: Vec<Window>,
}

impl Snapshot {
    async fn load(state: &AppState) -> Result<Self, AppError> {
        let stages = db::pickem::list_stages(&state.db).await?;
        let matches = db::pickem::list_matches(&state.db).await?;
        let potentials = db::pickem::list_potentials(&state.db).await?;
        let lobbies = db::pickem::list_lobby_conclusions(&state.db).await?;
        let windows = pickem::windows(&stages, &matches, &lobbies, OffsetDateTime::now_utc());
        Ok(Snapshot {
            stages,
            matches,
            potentials,
            windows,
        })
    }

    // Bracket matches and potential matches, for whatever reads them alike.
    fn every_match(&self) -> impl Iterator<Item = &MatchRow> {
        self.matches.iter().chain(&self.potentials)
    }

    fn window(&self, stage_id: Option<Uuid>) -> Option<&Window> {
        self.windows.iter().find(|w| Some(w.stage_id) == stage_id)
    }

    fn stage(&self, id: Uuid) -> Option<&StageRow> {
        self.stages.iter().find(|s| s.id == id)
    }
}

// Whether the viewer can predict, and if not, what's missing.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanPredict {
    Yes,
    SignIn,
    Discord,
    Banned,
}

fn can_predict(user: Option<&AuthUser>) -> CanPredict {
    match user {
        None => CanPredict::SignIn,
        Some(u) if u.banned => CanPredict::Banned,
        Some(u) if !u.discord_verified => CanPredict::Discord,
        Some(_) => CanPredict::Yes,
    }
}

#[derive(Debug, Serialize)]
pub struct PickemStage {
    id: Uuid,
    name: String,
    position: i32,
    best_of: Option<i32>,
    mode: Mode,
    status: Status,
    #[serde(with = "time::serde::rfc3339::option")]
    opens_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    closes_at: Option<OffsetDateTime>,
    // The stage it waits for.
    previous: Option<String>,
    // How many people have predicted at least one of its matches.
    pickers: usize,
}

#[derive(Debug, Serialize)]
pub struct MyPrediction {
    match_id: Uuid,
    team_id: Uuid,
    winner_score: i32,
    loser_score: i32,
    // What it scored, once the match has a result.
    points: Option<i32>,
    // The team it's on has been taken out of the match, so it can't score.
    void: bool,
}

fn my_prediction(p: &PredictionRow, m: &MatchRow) -> MyPrediction {
    let void = !pickem::counts(p, m);
    MyPrediction {
        match_id: p.match_id,
        team_id: p.team_id,
        winner_score: p.winner_score,
        loser_score: p.loser_score,
        points: pickem::outcome(m).map(|o| if void { 0 } else { pickem::points(p, &o) }),
        void,
    }
}

#[derive(Debug, Serialize)]
pub struct ScoreCount {
    // The team picked to win: 1 or 2.
    slot: u8,
    winner_score: i32,
    loser_score: i32,
    count: usize,
}

// How a locked match was predicted: how many picked each team, and each score.
#[derive(Debug, Serialize)]
pub struct Split {
    match_id: Uuid,
    total: usize,
    team1: usize,
    team2: usize,
    scores: Vec<ScoreCount>,
}

fn split(m: &MatchRow, predictions: &[&PredictionRow]) -> Split {
    let mut team1 = 0;
    let mut team2 = 0;
    let mut scores: HashMap<(u8, i32, i32), usize> = HashMap::new();
    for p in predictions.iter().filter(|p| pickem::counts(p, m)) {
        let slot = if Some(p.team_id) == m.team1_id { 1 } else { 2 };
        if slot == 1 {
            team1 += 1;
        } else {
            team2 += 1;
        }
        *scores
            .entry((slot, p.winner_score, p.loser_score))
            .or_default() += 1;
    }
    let mut scores: Vec<ScoreCount> = scores
        .into_iter()
        .map(|((slot, winner_score, loser_score), count)| ScoreCount {
            slot,
            winner_score,
            loser_score,
            count,
        })
        .collect();
    scores.sort_by_key(|s| (s.slot, s.loser_score));
    Split {
        match_id: m.id,
        total: team1 + team2,
        team1,
        team2,
        scores,
    }
}

#[derive(Debug, Serialize)]
pub struct StagePoints {
    stage_id: Uuid,
    #[serde(flatten)]
    tally: Tally,
}

#[derive(Debug, Serialize)]
pub struct Standing {
    // Shared by everyone level on points and exact predictions.
    rank: usize,
    player: Player,
    #[serde(flatten)]
    total: Tally,
    stages: Vec<StagePoints>,
}

fn standings(
    players: Vec<Player>,
    predictions: &[PredictionRow],
    matches: &HashMap<Uuid, &MatchRow>,
) -> Vec<Standing> {
    let mut tallies = pickem::tallies(predictions, matches);
    let mut rows: Vec<Standing> = players
        .into_iter()
        .filter_map(|player| {
            let (total, stages) = tallies.remove(&player.id)?;
            let mut stages: Vec<StagePoints> = stages
                .into_iter()
                .map(|(stage_id, tally)| StagePoints { stage_id, tally })
                .collect();
            stages.sort_by_key(|s| s.stage_id);
            Some(Standing {
                rank: 0,
                player,
                total,
                stages,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        (b.total.points, b.total.exact, b.total.winners)
            .cmp(&(a.total.points, a.total.exact, a.total.winners))
            .then_with(|| {
                a.player
                    .username
                    .to_lowercase()
                    .cmp(&b.player.username.to_lowercase())
            })
    });
    for i in 0..rows.len() {
        rows[i].rank = if i > 0
            && (rows[i].total.points, rows[i].total.exact)
                == (rows[i - 1].total.points, rows[i - 1].total.exact)
        {
            rows[i - 1].rank
        } else {
            i + 1
        };
    }
    rows
}

#[derive(Debug, Serialize)]
pub struct Overview {
    stages: Vec<PickemStage>,
    can_predict: CanPredict,
    // The viewer's own predictions, whatever the stage, potential matches included.
    mine: Vec<MyPrediction>,
    // Every match with both teams whose predictions are past changing (its stage is locked,
    // or it's locked on its own in a stage a host opened), potential matches included.
    splits: Vec<Split>,
    standings: Vec<Standing>,
}

// GET /api/public/pickem — every stage's pick'em, the viewer's predictions, how locked
// matches were predicted and the standings. Matches and teams come from the bracket.
async fn overview(
    State(state): State<AppState>,
    user: Option<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    let snap = Snapshot::load(&state).await?;
    let now = OffsetDateTime::now_utc();
    let predictions = db::pickem::list_predictions(&state.db).await?;
    let players = db::pickem::list_pickers(&state.db).await?;
    let matches: HashMap<Uuid, &MatchRow> = snap.matches.iter().map(|m| (m.id, m)).collect();
    // With the potential matches, which take predictions but never score.
    let every: HashMap<Uuid, &MatchRow> = snap.every_match().map(|m| (m.id, m)).collect();

    let mut by_match: HashMap<Uuid, Vec<&PredictionRow>> = HashMap::new();
    for p in &predictions {
        by_match.entry(p.match_id).or_default().push(p);
    }

    let stages = snap
        .windows
        .iter()
        .filter_map(|w| {
            let stage = snap.stage(w.stage_id)?;
            let pickers: HashSet<Uuid> = predictions
                .iter()
                .filter(|p| every.get(&p.match_id).and_then(|m| m.stage_id) == Some(stage.id))
                .map(|p| p.user_id)
                .collect();
            Some(PickemStage {
                id: stage.id,
                name: stage.name.clone(),
                position: stage.position,
                best_of: stage.best_of,
                mode: stage.pickem_mode,
                status: w.status,
                opens_at: w.opens_at,
                closes_at: w.closes_at,
                previous: w
                    .previous
                    .and_then(|id| snap.stage(id))
                    .map(|s| s.name.clone()),
                pickers: pickers.len(),
            })
        })
        .collect();

    let mine = match &user {
        Some(user) => predictions
            .iter()
            .filter(|p| p.user_id == user.id)
            .filter_map(|p| Some(my_prediction(p, every.get(&p.match_id)?)))
            .collect(),
        None => Vec::new(),
    };

    let splits = snap
        .every_match()
        .filter(|m| m.team1_id.is_some() && m.team2_id.is_some())
        .filter(|m| {
            snap.window(m.stage_id)
                .is_some_and(|w| pickem::revealed(w, m, now))
        })
        .map(|m| {
            split(
                m,
                by_match.get(&m.id).map(Vec::as_slice).unwrap_or_default(),
            )
        })
        .collect();

    Ok(Json(Overview {
        stages,
        can_predict: can_predict(user.as_ref()),
        mine,
        splits,
        standings: standings(players, &predictions, &matches),
    }))
}

#[derive(Debug, Serialize)]
pub struct MatchPickem {
    stage_id: Uuid,
    stage_name: String,
    // The stage's, except that a match in a stage a host opened is locked on its own once
    // its room opens or it has a result.
    status: Status,
    // When predictions on this match lock.
    #[serde(with = "time::serde::rfc3339::option")]
    closes_at: Option<OffsetDateTime>,
    // Whether the viewer could predict it now (their account aside).
    predictable: bool,
    can_predict: CanPredict,
    // How many have predicted it so far.
    total: usize,
    // How they predicted it, once it's locked.
    split: Option<Split>,
    mine: Option<MyPrediction>,
}

// GET /api/public/pickem/matches/:id — the public match page's pick'em panel; null for
// a match that isn't in a stage's pick'em (a lobby, or no stage).
async fn match_pickem(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let snap = Snapshot::load(&state).await?;
    let Some(m) = snap.matches.iter().find(|m| m.id == id) else {
        return Ok(Json(None));
    };
    let (Some(w), Some(stage)) = (
        snap.window(m.stage_id),
        m.stage_id.and_then(|s| snap.stage(s)),
    ) else {
        return Ok(Json(None));
    };
    let predictions = db::pickem::list_match_predictions(&state.db, id).await?;
    let refs: Vec<&PredictionRow> = predictions.iter().collect();
    let mine = user
        .as_ref()
        .and_then(|u| predictions.iter().find(|p| p.user_id == u.id))
        .map(|p| my_prediction(p, m));
    let has_teams = m.team1_id.is_some() && m.team2_id.is_some();
    let now = OffsetDateTime::now_utc();
    let status = if w.status == Status::Open && pickem::match_locked(m, w.mode, now) {
        Status::Locked
    } else {
        w.status
    };
    Ok(Json(Some(MatchPickem {
        stage_id: stage.id,
        stage_name: stage.name.clone(),
        status,
        // Opened by hand, each match locks on its own: at its time, if that's still to come
        // (it locks for good once its room opens).
        closes_at: if stage.pickem_mode == Mode::Open {
            m.starts_at.filter(|&t| t > now)
        } else {
            w.closes_at
        },
        predictable: pickem::predictable(w, m, stage.best_of, now),
        can_predict: can_predict(user.as_ref()),
        total: refs.iter().filter(|p| pickem::counts(p, m)).count(),
        split: (has_teams && pickem::revealed(w, m, now)).then(|| split(m, &refs)),
        mine,
    })))
}

// The match (or potential match), its stage's best of, if it can be predicted right
// now.
fn open_match(snap: &Snapshot, match_id: Uuid) -> Result<(&MatchRow, i32), AppError> {
    let m = snap
        .every_match()
        .find(|m| m.id == match_id)
        .ok_or(GeneralError::NotFound)?;
    let w = snap.window(m.stage_id).ok_or(GeneralError::NotFound)?;
    if w.status != Status::Open {
        return Err(PickemError::Closed.into());
    }
    let best_of = m
        .stage_id
        .and_then(|s| snap.stage(s))
        .and_then(|s| s.best_of);
    if !pickem::predictable(w, m, best_of, OffsetDateTime::now_utc()) {
        return Err(PickemError::CantPredict.into());
    }
    Ok((m, best_of.unwrap_or(1)))
}

#[derive(Debug, Deserialize)]
pub struct PredictBody {
    team_id: Uuid,
    // The losing team's maps; the winner's are what the best of needs.
    loser_score: i32,
}

// PUT /api/pickem/:match_id — the viewer's prediction for a match of an open stage,
// replacing any they made before.
async fn predict(
    State(state): State<AppState>,
    user: AuthUser,
    Path(match_id): Path<Uuid>,
    Json(body): Json<PredictBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_not_banned()?;
    user.require_discord_verified()?;
    let snap = Snapshot::load(&state).await?;
    let (m, best_of) = open_match(&snap, match_id)?;
    if Some(body.team_id) != m.team1_id && Some(body.team_id) != m.team2_id {
        return Err(MatchError::TeamNotInMatch.into());
    }
    let winner_score = pickem::first_to(best_of);
    if !(0..winner_score).contains(&body.loser_score) {
        return Err(PickemError::ImpossibleScore.into());
    }
    db::pickem::save_prediction(
        &state.db,
        &PredictionRow {
            user_id: user.id,
            match_id,
            team_id: body.team_id,
            winner_score,
            loser_score: body.loser_score,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/pickem/:match_id — takes the viewer's prediction back while it's open.
async fn withdraw(
    State(state): State<AppState>,
    user: AuthUser,
    Path(match_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let snap = Snapshot::load(&state).await?;
    open_match(&snap, match_id)?;
    db::pickem::delete_prediction(&state.db, user.id, match_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ModeBody {
    mode: Mode,
}

// PUT /api/stages/:id/pickem — host-only. Opens a stage's pick'em early, locks it, or
// hands it back to the rule.
async fn set_mode(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ModeBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::pickem::set_mode(&state.db, id, body.mode).await? {
        return Err(GeneralError::NotFound.into());
    }
    Ok(StatusCode::NO_CONTENT)
}
