//! The statistics page: a published stage's statistics, counted from the scores the site
//! saved (crate::stats). Public, like the stage's pool. What's counted is kept until the
//! site next changes (crate::stats_cache).

use std::collections::HashSet;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    db::{self, bracket::Side, mappool::PublicStageDetail, stats::StatMatch, team::Player},
    error::{AppError, GeneralError},
    services::{self, stats::Counted},
    stats::Stats,
    views::{group_teams, lobby_name, Team},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/stats", get(list_stage_activity))
        .route("/public/stats/{id}", get(get_stage_stats))
        .route("/public/qualifier-places", get(list_qualifier_places))
}

// The stats cache's key for the qualifier places (the nil id holds the stage activity;
// every other key is a stage's random id).
const QUALIFIER_PLACES_KEY: Uuid = Uuid::from_u128(1);

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

// GET /api/public/qualifier-places — each team's place in the qualifier, by team id,
// once the qualifier's stage is published (as /stats shows it). The team popovers read
// it, whichever page's team data opened them.
async fn list_qualifier_places(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let body = state
        .stats_cache
        .get_or_count(QUALIFIER_PLACES_KEY, || async {
            to_body(&services::stats::qualifier_places(&state.db, false).await?)
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
    if db::stages::get_published_stage(&state.db, id)
        .await?
        .is_none()
    {
        return Err(GeneralError::NotFound.into());
    }
    let body = state
        .stats_cache
        .get_or_count(id, || count_stage(&state, id))
        .await?;
    Ok(json(body))
}

// Counts a published stage's statistics from the database.
async fn count_stage(state: &AppState, id: Uuid) -> Result<Bytes, AppError> {
    let stage = db::stages::get_published_stage(&state.db, id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    let pool = services::stats::stage_pool(&state.db, id).await?;
    let Counted {
        matches,
        members,
        stats,
    } = services::stats::count(&state.db, id, stage.is_qualifier, &pool.order).await?;

    let shown: HashSet<Uuid> = stats.teams.iter().map(|t| t.id).collect();
    let teams = group_teams(members, |r| r.player)
        .into_iter()
        .filter(|t| shown.contains(&t.id))
        .collect();
    to_body(&StageStats {
        pool: PublicStageDetail {
            stage,
            categories: pool.categories,
            slots: pool.slots,
            maps: pool.maps,
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
