//! The bracket's seeding (/admin/seeding): the qualifier's standings as the order the
//! teams are seeded in, which hosts can rearrange, the double-elimination bracket made
//! from it in one go (crate::seeding), and taking that bracket down again while none of
//! it has been played.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

use crate::{
    db::{
        self,
        seeding::{BracketState, NewMatch},
        stages::Stage,
    },
    discord::jobs::Job,
    error::{AppError, BracketError},
    seeding::{self, BracketFormat},
    services::{self, bracket::deletion_jobs},
    views::{group_teams, lobby_name, AdminTeamPlayer, Team},
    AppState,
};

// What the qualifier says of a team.
#[derive(Debug, Serialize)]
pub struct SeedStanding {
    pub rank: usize,
    // The sum of its points (Z-percentiles) over the pool: what the qualifier ranks by.
    pub points: f64,
    pub total_score: i64,
    // Maps played, and maps counted as -1 (no score although the lobby is over).
    pub plays: usize,
    pub missing: usize,
    // Its lobby isn't over, so its missing maps don't count yet.
    pub pending: bool,
}

#[derive(Debug, Serialize)]
pub struct SeedTeam {
    #[serde(flatten)]
    pub team: Team<AdminTeamPlayer>,
    // None: no place in the qualifier (it played no lobby).
    pub qualifier: Option<SeedStanding>,
    // The qualifier lobby it signed up for ("B").
    pub lobby: Option<String>,
    // The hosts banned one of its players.
    pub banned: bool,
    // It plays in the bracket.
    pub in_bracket: bool,
    // Its seed in the qualifier's order (banned teams last), from 1: where it is unless
    // a host moved it.
    pub qualifier_seed: usize,
}

#[derive(Debug, Serialize)]
pub struct SeedingView {
    // In seeding order, best seed first.
    pub teams: Vec<SeedTeam>,
    // A host rearranged it: it's no longer the qualifier's order.
    pub custom: bool,
    pub qualifier_stage_id: Option<Uuid>,
    pub pool_size: usize,
    pub lobbies_not_over: i64,
    pub matches: i64,
    pub played: i64,
    // The bracket stages, in play order, for the weeks of a bracket made from it.
    pub stages: Vec<Stage>,
}

// The order the qualifier gives: its ranked teams by place, then the teams it didn't
// rank by team value, and the teams with a banned player last, whatever their place.
fn qualifier_order(view: &[SeedTeam]) -> Vec<Uuid> {
    let mut ids: Vec<&SeedTeam> = view.iter().collect();
    ids.sort_by(|a, b| {
        a.banned
            .cmp(&b.banned)
            .then_with(|| match (&a.qualifier, &b.qualifier) {
                (Some(x), Some(y)) => x.rank.cmp(&y.rank),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => {
                    let value = |t: &SeedTeam| t.team.team_value.unwrap_or(f64::NEG_INFINITY);
                    value(b).total_cmp(&value(a))
                }
            })
    });
    ids.into_iter().map(|t| t.team.id).collect()
}

pub async fn view(state: &AppState) -> Result<SeedingView, AppError> {
    let qualifier = services::stats::qualifier(&state.db, true).await?;
    let mut standings: HashMap<Uuid, SeedStanding> = HashMap::new();
    let mut lobbies: HashMap<Uuid, String> = HashMap::new();
    let (stage_id, rows) = match qualifier {
        Some((stage_id, counted)) => {
            for s in &counted.stats.teams {
                let Some(rank) = s.rank else { continue };
                standings.insert(
                    s.id,
                    SeedStanding {
                        rank,
                        points: s.points,
                        total_score: s.total_score,
                        plays: s.plays,
                        missing: s.per_map.iter().filter(|c| c.missing).count(),
                        pending: s.pending,
                    },
                );
            }
            for m in &counted.matches {
                let Some(index) = m.lobby_index else { continue };
                for team in &m.teams {
                    lobbies.insert(*team, lobby_name(index));
                }
            }
            (Some(stage_id), counted.members)
        }
        None => (None, db::team::list_team_members(&state.db, None).await?),
    };
    let pool_size = match stage_id {
        Some(id) => services::stats::stage_pool(&state.db, id)
            .await?
            .order
            .len(),
        None => 0,
    };

    let in_bracket: HashSet<Uuid> = db::seeding::bracket_teams(&state.db)
        .await?
        .into_iter()
        .collect();
    let mut teams: Vec<SeedTeam> = group_teams(rows, AdminTeamPlayer::from_row)
        .into_iter()
        .map(|team| SeedTeam {
            qualifier: standings.remove(&team.id),
            lobby: lobbies.remove(&team.id),
            banned: team.players.iter().any(|p| p.banned()),
            in_bracket: in_bracket.contains(&team.id),
            qualifier_seed: 0,
            team,
        })
        .collect();

    let default = qualifier_order(&teams);
    for team in &mut teams {
        team.qualifier_seed = default
            .iter()
            .position(|id| *id == team.team.id)
            .map_or(0, |i| i + 1);
    }
    let saved = db::seeding::saved_order(&state.db).await?;
    let order = seeding::order(&default, &saved);
    let place: HashMap<Uuid, usize> = order.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    teams.sort_by_key(|t| place.get(&t.team.id).copied().unwrap_or(usize::MAX));

    let BracketState { matches, played } = db::seeding::bracket_state(&state.db).await?;
    Ok(SeedingView {
        teams,
        custom: !saved.is_empty() && order != default,
        qualifier_stage_id: stage_id,
        pool_size,
        lobbies_not_over: db::seeding::lobbies_not_over(&state.db).await?,
        matches,
        played,
        stages: db::stages::list_stages(&state.db)
            .await?
            .into_iter()
            .filter(|s| !s.is_qualifier)
            .collect(),
    })
}

// Each team once, and every one of them a team.
async fn check_order(state: &AppState, team_ids: &[Uuid]) -> Result<(), AppError> {
    let distinct: HashSet<&Uuid> = team_ids.iter().collect();
    if distinct.len() != team_ids.len() {
        return Err(BracketError::BadSeeding.into());
    }
    if db::seeding::existing_teams(&state.db, team_ids).await? != team_ids.len() as i64 {
        return Err(BracketError::UnknownTeam.into());
    }
    Ok(())
}

// Saves a host's order, or (None) hands the seeding back to the qualifier's. Once a
// bracket is made from it, it's the record of who got which seed, so it stays.
pub async fn save_order(state: &AppState, team_ids: Option<&[Uuid]>) -> Result<(), AppError> {
    if let Some(ids) = team_ids {
        check_order(state, ids).await?;
    }
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    if db::seeding::bracket_state(&mut *tx).await?.matches > 0 {
        return Err(BracketError::NotEmpty.into());
    }
    match team_ids {
        Some(ids) => db::seeding::save_order(&mut tx, ids).await?,
        None => db::seeding::clear_order(&mut *tx).await?,
    }
    tx.commit().await?;
    Ok(())
}

// Makes the whole bracket from the seeding: `team_ids` is the order (every team the
// host sees, best seed first), of which the format takes the top; `stage_ids` the stage
// of each of its weeks. Only into an empty bracket, once every lobby is over.
pub async fn make_bracket(
    state: &AppState,
    format: BracketFormat,
    team_ids: &[Uuid],
    stage_ids: &[Option<Uuid>],
) -> Result<(), AppError> {
    if team_ids.len() < format.teams() {
        return Err(BracketError::WrongTeamCount.into());
    }
    check_order(state, team_ids).await?;
    if stage_ids.len() != format.weeks() {
        return Err(BracketError::BadStages.into());
    }
    let stages: Vec<Uuid> = stage_ids.iter().flatten().copied().collect();
    let distinct: HashSet<&Uuid> = stages.iter().collect();
    if distinct.len() != stages.len()
        || db::seeding::existing_bracket_stages(&state.db, &stages).await? != stages.len() as i64
    {
        return Err(BracketError::BadStages.into());
    }

    let plan = seeding::plan(format);
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    if db::seeding::bracket_state(&mut *tx).await?.matches > 0 {
        return Err(BracketError::NotEmpty.into());
    }
    if db::seeding::lobbies_not_over(&mut *tx).await? > 0 {
        return Err(BracketError::QualifiersNotOver.into());
    }
    let mut ids = Vec::with_capacity(plan.matches.len());
    let mut jobs = Vec::new();
    for m in &plan.matches {
        let seed = |s: usize| team_ids[s - 1];
        let id = db::seeding::insert_match(
            &mut tx,
            &NewMatch {
                side: m.side,
                number: m.number,
                pos_x: m.pos_x,
                pos_y: m.pos_y,
                team1_id: m.seeds.map(|(a, _)| seed(a)),
                team2_id: m.seeds.map(|(_, b)| seed(b)),
                stage_id: stage_ids[m.week],
            },
        )
        .await?;
        if m.seeds.is_some() {
            jobs.push(Job::SyncMatch { match_id: id });
        }
        ids.push(id);
    }
    for l in &plan.links {
        db::seeding::insert_link(&mut tx, ids[l.source], l.outcome, ids[l.target], l.slot).await?;
    }
    db::seeding::save_order(&mut tx, team_ids).await?;
    jobs.extend(db::potential::sync(&mut tx, &[]).await?);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(())
}

// Deletes every bracket match, and their potential matches, while none has been played
// (concluded or given a room). The seeding stays, so the same bracket can be made again.
pub async fn remove_bracket(state: &AppState) -> Result<(), AppError> {
    let mut tx = state.db.begin().await?;
    db::potential::lock(&mut tx).await?;
    let ids = db::seeding::lock_bracket(&mut tx).await?;
    if db::seeding::bracket_state(&mut *tx).await?.played > 0 {
        return Err(BracketError::AlreadyPlayed.into());
    }
    let mut jobs = Vec::new();
    for &id in &ids {
        jobs.extend(db::potential::remove_of_match(&mut tx, id).await?);
        jobs.extend(deletion_jobs(&mut tx, id).await?);
    }
    db::seeding::delete_bracket(&mut tx).await?;
    jobs.extend(db::potential::sync(&mut tx, &[]).await?);
    state.discord.enqueue(&mut *tx, &jobs).await?;
    tx.commit().await?;
    state.live.bracket_changed();
    Ok(())
}
