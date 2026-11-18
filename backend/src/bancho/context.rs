//! What the bots need to know about a match to run and count it: its players and where
//! they belong, its pool, what its referee marked, the room options and the score the
//! site has. Loaded from the database again whenever any of it changes.

use std::collections::HashSet;

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    commands::{side_color, side_slots, ROOM_PREFIX},
    tally::{lobby_key, pool_entries, score_key, PoolEntry, RosterPlayer},
};
use crate::{
    db::{
        self,
        bracket::{BanRow, ManualResultRow, MapResultRow, PickRow, Side},
        matches::MatchKind,
        referee::RoomSettings,
    },
    draft::DraftOrder,
    views::lobby_name,
};

#[derive(Debug, Clone)]
pub struct MatchTeam {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MatchContext {
    pub kind: MatchKind,
    pub starts_at: Option<OffsetDateTime>,
    pub concluded_at: Option<OffsetDateTime>,
    pub forfeit: bool,
    // "W3", "Lobi B": how the bots page names it.
    pub label: String,
    // The osu! room name: "112S: (Team A) vs (Team B)", or "112S: (Qualifiers) vs
    // (Lobby B)" for a lobby.
    pub room_name: String,
    // Bracket matches: team 1 (blue) and team 2 (red), once both are known.
    pub sides: Option<[MatchTeam; 2]>,
    // Every team of the match: a bracket match's two in slot order, a lobby's in signup
    // order.
    pub teams: Vec<MatchTeam>,
    pub roster: Vec<RosterPlayer>,
    pub best_of: Option<i32>,
    // The stage's pick and ban order (the default when it has none).
    pub draft_order: DraftOrder,
    pub entries: Vec<PoolEntry>,
    pub bans: Vec<BanRow>,
    pub picks: Vec<PickRow>,
    pub manual: Vec<ManualResultRow>,
    pub ignored: HashSet<String>,
    pub settings: RoomSettings,
    // The score the site has, as tally::score_key reads it; a lobby's saved results, as
    // tally::lobby_key reads them.
    pub saved_key: String,
}

impl MatchContext {
    pub fn concluded(&self) -> bool {
        self.concluded_at.is_some()
    }

    pub fn is_bracket(&self) -> bool {
        self.kind == MatchKind::Bracket
    }
}

fn code(side: Option<Side>, number: Option<i32>) -> String {
    match (side, number) {
        (Some(Side::Winners), Some(n)) => format!("W{n}"),
        (Some(Side::Losers), Some(n)) => format!("L{n}"),
        _ => "Maç".into(),
    }
}

// The match as the bots run it; None when it doesn't exist.
pub async fn load(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<MatchContext>> {
    let Some(row) = db::matches::find_room(pool, id).await? else {
        return Ok(None);
    };
    let bracket = row.kind == MatchKind::Bracket;

    // A bracket match's two teams, or a lobby's in signup order.
    let order: Vec<Uuid> = if bracket {
        [row.team1_id, row.team2_id].into_iter().flatten().collect()
    } else {
        db::matches::list_entries(pool)
            .await?
            .into_iter()
            .filter(|e| e.match_id == id)
            .map(|e| e.team_id)
            .collect()
    };
    let members = db::team::list_team_members(pool, None).await?;
    let mut roster = Vec::new();
    let mut names = Vec::new();
    for team_id in &order {
        let side = match bracket {
            true if Some(*team_id) == row.team1_id => Some(1),
            true if Some(*team_id) == row.team2_id => Some(2),
            _ => None,
        };
        let players: Vec<_> = members.iter().filter(|m| m.team_id == *team_id).collect();
        if let Some(first) = players.first() {
            names.push(MatchTeam {
                id: *team_id,
                name: first.team_name.clone(),
            });
        }
        for (i, m) in players.iter().enumerate() {
            roster.push(RosterPlayer {
                user_id: m.player.id,
                osu_id: m.player.osu_id,
                username: m.player.username.clone(),
                team_id: *team_id,
                color: side.map(side_color),
                home: side.and_then(|s| side_slots(s).get(i).copied()),
            });
        }
    }
    let team = |id: Option<Uuid>| id.and_then(|id| names.iter().find(|t| t.id == id).cloned());
    let sides = match (team(row.team1_id), team(row.team2_id)) {
        (Some(a), Some(b)) if bracket => Some([a, b]),
        _ => None,
    };
    let lobby = lobby_name(row.lobby_index.unwrap_or_default());
    let room_name = if bracket {
        let name = |id: Option<Uuid>| team(id).map_or("TBD".to_string(), |t| t.name);
        format!(
            "{ROOM_PREFIX}: ({}) vs ({})",
            name(row.team1_id),
            name(row.team2_id)
        )
    } else {
        format!("{ROOM_PREFIX}: (Qualifiers) vs (Lobby {lobby})")
    };

    let entries = match row.stage_id {
        Some(stage) => pool_entries(
            &db::mappool::list_categories(pool, stage).await?,
            &db::mappool::list_public_slots(pool, stage).await?,
            &db::mappool::list_public_maps(pool, stage).await?,
        ),
        None => Vec::new(),
    };
    let (bans, picks, manual, map_results) = if bracket {
        (
            db::bracket::list_bans(pool, id).await?,
            db::bracket::list_picks(pool, id).await?,
            db::bracket::list_manual_results(pool, id).await?,
            db::bracket::list_map_results(pool, id).await?,
        )
    } else {
        (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::<MapResultRow>::new(),
        )
    };

    Ok(Some(MatchContext {
        kind: row.kind,
        starts_at: row.starts_at,
        concluded_at: row.concluded_at,
        forfeit: row.forfeit_slot.is_some(),
        label: if bracket {
            code(row.side, row.number)
        } else {
            format!("Lobi {lobby}")
        },
        room_name,
        sides,
        teams: names,
        roster,
        best_of: row.stage_best_of,
        draft_order: DraftOrder::of_stage(row.stage_draft_order.as_deref()),
        entries,
        bans,
        picks,
        manual,
        ignored: db::referee::list_ignored(pool, id)
            .await?
            .into_iter()
            .collect(),
        settings: db::referee::find_settings(pool, id).await?,
        saved_key: if bracket {
            score_key(row.team1_score, row.team2_score, &map_results)
        } else {
            lobby_key(&db::stats::list_qualifier_results(pool, id).await?)
        },
    }))
}
