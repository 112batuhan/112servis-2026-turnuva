//! The statistics page: a published stage's statistics, counted from the scores the site
//! saved (crate::stats). Public, like the stage's pool. Every open statistics page reads
//! them again after each saved map, so the result is kept (StatsCache) until the site
//! next changes: one count per change, however many are watching.

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use tokio::{
    sync::broadcast,
    time::{Duration, Instant},
};
use uuid::Uuid;

use crate::{
    db::{
        self,
        bracket::Side,
        mappool::PublicStageDetail,
        stats::{StatMatch, StatScore},
        team::Player,
    },
    error::AppError,
    handlers::{
        matches::lobby_name,
        team::{group_teams, Team},
    },
    stats::{self, DraftIn, Input, MatchIn, ScoreIn, Stats},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/stats", get(list_stage_activity))
        .route("/public/stats/:id", get(get_stage_stats))
}

// What's counted is kept this long at most, for the changes the live hub doesn't hear of
// (a pool edited, a team renamed).
const CACHE_TTL: Duration = Duration::from_secs(60);

// The statistics last counted, by stage (the stages' activity under the nil id), as the
// JSON sent. An entry stands until the live hub hears of any change (a saved score, a
// lobby's maps, accuracy from osu!, the bracket) or CACHE_TTL passes. A stage is counted
// by one request at a time: those arriving meanwhile wait for it and take its result.
#[derive(Clone, Default)]
pub struct StatsCache(Arc<CacheInner>);

#[derive(Default)]
struct CacheInner {
    // Bumped by every change.
    generation: AtomicU64,
    entries: Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<Option<CacheEntry>>>>>,
}

struct CacheEntry {
    generation: u64,
    at: Instant,
    body: Bytes,
}

impl StatsCache {
    async fn get_or_count<F, Fut>(&self, key: Uuid, count: F) -> Result<Bytes, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Bytes, AppError>>,
    {
        let slot = self
            .0
            .entries
            .lock()
            .expect("stats cache poisoned")
            .entry(key)
            .or_default()
            .clone();
        let mut entry = slot.lock().await;
        // Taken before reading, so a change made while counting makes the next request
        // count again.
        let generation = self.0.generation.load(Ordering::Acquire);
        if let Some(e) = entry.as_ref() {
            if e.generation == generation && e.at.elapsed() < CACHE_TTL {
                return Ok(e.body.clone());
            }
        }
        let body = count().await?;
        *entry = Some(CacheEntry {
            generation,
            at: Instant::now(),
            body: body.clone(),
        });
        Ok(body)
    }

    fn invalidate(&self) {
        self.0.generation.fetch_add(1, Ordering::AcqRel);
    }
}

// Drops what's counted whenever the live hub hears of a change.
pub fn spawn_invalidation(state: &AppState) {
    let cache = state.stats_cache.clone();
    let mut changes = state.live.changes();
    tokio::spawn(async move {
        loop {
            match changes.recv().await {
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => cache.invalidate(),
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

fn json(body: Bytes) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], body)
}

fn to_body<T: Serialize>(value: &T) -> Result<Bytes, AppError> {
    Ok(Bytes::from(serde_json::to_vec(value)?))
}

// GET /api/public/stats — which published stages have scores, and which are being played.
async fn list_stage_activity(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let body = state
        .stats_cache
        .get_or_count(Uuid::nil(), || async {
            to_body(&db::stats::stage_activity(&state.db).await?)
        })
        .await?;
    Ok(json(body))
}

#[derive(Debug, Serialize)]
struct MatchView {
    id: Uuid,
    // "W3", "L7", "Lobi B".
    code: String,
    side: Option<Side>,
    concluded: bool,
    // It has a room and isn't concluded: its scores are still coming in.
    live: bool,
}

#[derive(Debug, Serialize)]
struct StageStats {
    // The stage and its pool, as the public pool page has them.
    pool: PublicStageDetail,
    matches: Vec<MatchView>,
    // Every team in the statistics, with its players.
    teams: Vec<Team<Player>>,
    stats: Stats,
}

fn code(m: &StatMatch) -> String {
    match (m.side, m.number, m.lobby_index) {
        (Some(Side::Winners), Some(n), _) => format!("W{n}"),
        (Some(Side::Losers), Some(n), _) => format!("L{n}"),
        (_, _, Some(i)) => format!("Lobi {}", lobby_name(i)),
        _ => "Maç".into(),
    }
}

// GET /api/public/stats/:id — a published stage's statistics. Whether it's published is
// asked every time, so a stage taken off the site is never served from the cache.
async fn get_stage_stats(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if db::mappool::get_published_stage(&state.db, id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound);
    }
    let body = state
        .stats_cache
        .get_or_count(id, || count_stage(&state, id))
        .await?;
    Ok(json(body))
}

// Counts a published stage's statistics from the database.
async fn count_stage(state: &AppState, id: Uuid) -> Result<Bytes, AppError> {
    let stage = db::mappool::get_published_stage(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let qualifier = stage.is_qualifier;
    let categories = db::mappool::list_categories(&state.db, id).await?;
    let slots = db::mappool::list_public_slots(&state.db, id).await?;
    let maps = db::mappool::list_public_maps(&state.db, id).await?;

    // The pool in category and slot order.
    let mut order: Vec<Uuid> = Vec::new();
    for c in &categories {
        let mut own: Vec<_> = slots.iter().filter(|s| s.category_id == c.id).collect();
        own.sort_by_key(|s| s.position);
        for slot in own {
            if let Some(m) = maps.iter().find(|m| m.slot_id == Some(slot.id)) {
                order.push(m.id);
            }
        }
    }

    let (matches, scores, drafts) = if qualifier {
        (
            db::stats::qualifier_lobbies(&state.db).await?,
            db::stats::qualifier_scores(&state.db, id).await?,
            Vec::new(),
        )
    } else {
        (
            db::stats::bracket_matches(&state.db, id).await?,
            db::stats::bracket_scores(&state.db, id).await?,
            db::stats::drafts(&state.db, id).await?,
        )
    };

    let rows = db::team::list_team_members(&state.db, None).await?;
    let mut members: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for r in &rows {
        members.entry(r.team_id).or_default().push(r.player.id);
    }

    let stats = stats::compute(&Input {
        qualifier,
        maps: &order,
        matches: &matches
            .iter()
            .map(|m| MatchIn {
                id: m.id,
                concluded: m.concluded_at.is_some(),
                teams: m.teams.clone(),
            })
            .collect::<Vec<_>>(),
        scores: &scores.iter().map(score_in).collect::<Vec<_>>(),
        drafts: &drafts
            .iter()
            .map(|d| DraftIn {
                match_id: d.match_id,
                map_id: d.map_id,
                team_id: d.team_id,
                ban: d.ban,
            })
            .collect::<Vec<_>>(),
        members: &members,
    });

    let shown: HashSet<Uuid> = stats.teams.iter().map(|t| t.id).collect();
    let teams = group_teams(rows, |r| r.player)
        .into_iter()
        .filter(|t| shown.contains(&t.id))
        .collect();
    to_body(&StageStats {
        pool: PublicStageDetail {
            stage,
            categories,
            slots,
            maps,
        },
        matches: matches
            .iter()
            .map(|m| MatchView {
                id: m.id,
                code: code(m),
                side: m.side,
                concluded: m.concluded_at.is_some(),
                live: m.has_room && m.concluded_at.is_none(),
            })
            .collect(),
        teams,
        stats,
    })
}

fn score_in(s: &StatScore) -> ScoreIn {
    ScoreIn {
        match_id: s.match_id,
        position: s.position,
        map_id: s.map_id,
        user_id: s.user_id,
        team_id: s.team_id,
        score: s.score,
        accuracy: s.accuracy,
        mods: s.mods.clone(),
        passed: s.passed,
    }
}
