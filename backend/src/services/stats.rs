//! Counting a stage's statistics (crate::stats) from the scores the site saved: for the
//! statistics page, and the qualifier's for the seeding (the seeding page and the team
//! lists, which show each team's place in it).

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    db::{
        self,
        mappool::{Category, PublicPoolMap, PublicSlot},
        stats::{StatMatch, StatScore},
        team::TeamMemberRow,
    },
    stats::{self, DraftIn, Input, MatchIn, ScoreIn, Stats},
};

// A stage's pool, and its maps in category and slot order (the order the statistics
// count them in).
pub struct StagePool {
    pub categories: Vec<Category>,
    pub slots: Vec<PublicSlot>,
    pub maps: Vec<PublicPoolMap>,
    pub order: Vec<Uuid>,
}

pub async fn stage_pool(db: &PgPool, stage_id: Uuid) -> sqlx::Result<StagePool> {
    let categories = db::mappool::list_categories(db, stage_id).await?;
    let slots = db::mappool::list_public_slots(db, stage_id).await?;
    let maps = db::mappool::list_public_maps(db, stage_id).await?;
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
    Ok(StagePool {
        categories,
        slots,
        maps,
        order,
    })
}

// What a stage's statistics were counted from, with what they came to.
pub struct Counted {
    pub matches: Vec<StatMatch>,
    // Every team's players, ordered by team.
    pub members: Vec<TeamMemberRow>,
    pub stats: Stats,
}

pub async fn count(
    db: &PgPool,
    stage_id: Uuid,
    qualifier: bool,
    order: &[Uuid],
) -> sqlx::Result<Counted> {
    let (matches, scores, drafts) = if qualifier {
        (
            db::stats::qualifier_lobbies(db).await?,
            db::stats::qualifier_scores(db, stage_id).await?,
            Vec::new(),
        )
    } else {
        (
            db::stats::bracket_matches(db, stage_id).await?,
            db::stats::bracket_scores(db, stage_id).await?,
            db::stats::drafts(db, stage_id).await?,
        )
    };

    let members = db::team::list_team_members(db, None).await?;
    let mut by_team: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for r in &members {
        by_team.entry(r.team_id).or_default().push(r.player.id);
    }

    let stats = stats::compute(&Input {
        qualifier,
        maps: order,
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
                opponent_id: d.opponent_id,
                ban: d.ban,
            })
            .collect::<Vec<_>>(),
        members: &by_team,
    });
    Ok(Counted {
        matches,
        members,
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

// The qualifier's statistics, when there's a qualifier stage (only a published one
// unless `unpublished` too: the public pages only show what /stats shows).
pub async fn qualifier(db: &PgPool, unpublished: bool) -> sqlx::Result<Option<(Uuid, Counted)>> {
    let Some(stage_id) = db::stages::qualifier_stage_id(db, unpublished).await? else {
        return Ok(None);
    };
    let pool = stage_pool(db, stage_id).await?;
    Ok(Some((
        stage_id,
        count(db, stage_id, true, &pool.order).await?,
    )))
}

// A team's place in the qualifier, as the team lists show it.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct QualifierPlace {
    pub rank: usize,
    // Its lobby isn't over, so the place can still change.
    pub pending: bool,
    // A bracket was made and it isn't in it: the seeding's cut left it out of the
    // tournament.
    pub eliminated: bool,
}

pub async fn qualifier_places(
    db: &PgPool,
    unpublished: bool,
) -> sqlx::Result<HashMap<Uuid, QualifierPlace>> {
    let Some((_, counted)) = qualifier(db, unpublished).await? else {
        return Ok(HashMap::new());
    };
    let in_bracket: HashSet<Uuid> = db::seeding::bracket_teams(db).await?.into_iter().collect();
    Ok(counted
        .stats
        .teams
        .iter()
        .filter_map(|s| {
            s.rank.map(|rank| {
                (
                    s.id,
                    QualifierPlace {
                        rank,
                        pending: s.pending,
                        eliminated: !in_bracket.is_empty() && !in_bracket.contains(&s.id),
                    },
                )
            })
        })
        .collect())
}
