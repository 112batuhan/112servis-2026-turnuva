//! The score counted from the finished maps, and the rows saved from them.

use std::collections::HashSet;

use uuid::Uuid;

use crate::bancho::room::MapResult;
use crate::db::{
    bracket::{ManualResultRow, MapPlayerScore, MapResultRow, PickRow},
    stats::QualifierMapRow,
};

use super::{entry_of, scorer, team_totals, Finished, PoolEntry, RosterPlayer, TeamTotal};

// A finished map of the pool, with who won it.
#[derive(Debug, Clone)]
pub struct Row {
    pub key: String,
    pub map_id: Uuid,
    pub totals: Vec<TeamTotal>,
    // The team that won it, or None for a tie or a map without scores.
    pub winner_id: Option<Uuid>,
}

// A finished map read against the pool: what the score sheet lists for it.
pub fn row_of(
    f: &Finished,
    entries: &[PoolEntry],
    picked: &HashSet<Uuid>,
    roster: &[RosterPlayer],
) -> Option<Row> {
    let entry = entry_of(
        entries,
        f.result.beatmap_id,
        &f.result.room_mods,
        f.result.freemod,
        picked,
    )?;
    let totals = team_totals(&f.result, roster);
    let winner_id = match totals.as_slice() {
        [best, second, ..] if second.score == best.score => None,
        [best, ..] => Some(best.team_id),
        [] => None,
    };
    Some(Row {
        key: f.key.clone(),
        map_id: entry.map_id,
        totals,
        winner_id,
    })
}

// A bracket match's map wins, from its rooms and the referee's hand-awarded results.
#[derive(Debug, Clone)]
pub struct Tally {
    // The counted rows as the site saves them with the score.
    pub maps: Vec<MapResultRow>,
    pub team1: i32,
    pub team2: i32,
}

pub struct TallyInput<'a> {
    pub finished: &'a [Finished],
    pub entries: &'a [PoolEntry],
    pub picks: &'a [PickRow],
    pub roster: &'a [RosterPlayer],
    pub ignored: &'a HashSet<String>,
    pub manual: &'a [ManualResultRow],
    pub team1_id: Uuid,
    pub team2_id: Uuid,
}

pub fn tally(input: &TallyInput) -> Tally {
    let picked: HashSet<Uuid> = input.picks.iter().map(|p| p.map_id).collect();
    let counted: Vec<(Row, &Finished)> = input
        .finished
        .iter()
        .filter_map(|f| row_of(f, input.entries, &picked, input.roster).map(|r| (r, f)))
        .filter(|(r, _)| !input.ignored.contains(&r.key))
        .collect();
    let wins = |team: Uuid| {
        (counted
            .iter()
            .filter(|(r, _)| r.winner_id == Some(team))
            .count()
            + input.manual.iter().filter(|m| m.team_id == team).count()) as i32
    };
    let total = |r: &Row, team: Uuid| {
        r.totals
            .iter()
            .find(|t| t.team_id == team)
            .map_or(0, |t| t.score)
    };
    Tally {
        maps: counted
            .iter()
            .map(|(r, f)| MapResultRow {
                map_id: Some(r.map_id),
                team1_score: total(r, input.team1_id),
                team2_score: total(r, input.team2_id),
                players: player_scores(&f.result, input.roster),
            })
            .collect(),
        team1: wins(input.team1_id),
        team2: wins(input.team2_id),
    }
}

// The scores of a finished map by the match's players, one per player.
pub fn player_scores(result: &MapResult, roster: &[RosterPlayer]) -> Vec<MapPlayerScore> {
    let mut seen = HashSet::new();
    result
        .scores
        .iter()
        .filter_map(|s| {
            scorer(s, roster)
                .filter(|p| seen.insert(p.user_id))
                .map(|p| MapPlayerScore {
                    user_id: p.user_id,
                    score: s.score,
                    mods: s.mods.clone(),
                    passed: s.passed,
                })
        })
        .collect()
}

// A qualifier lobby's finished pool maps as the site saves them: every one the referee
// didn't leave out ("Sayma"), in the order played, attempts at the same map included,
// with the scores of the lobby's players.
pub fn lobby_rows(
    finished: &[Finished],
    entries: &[PoolEntry],
    roster: &[RosterPlayer],
    ignored: &HashSet<String>,
) -> Vec<QualifierMapRow> {
    let none = HashSet::new();
    finished
        .iter()
        .filter(|f| !ignored.contains(&f.key))
        .filter_map(|f| {
            let entry = entry_of(
                entries,
                f.result.beatmap_id,
                &f.result.room_mods,
                f.result.freemod,
                &none,
            )?;
            let players = player_scores(&f.result, roster);
            (!players.is_empty()).then_some(QualifierMapRow {
                map_id: Some(entry.map_id),
                players,
            })
        })
        .collect()
}

// What a lobby's saved results are compared by.
pub fn lobby_key(rows: &[QualifierMapRow]) -> String {
    let rows: Vec<_> = rows
        .iter()
        .map(|r| {
            let mut players: Vec<_> = r
                .players
                .iter()
                .map(|p| (p.user_id, p.score, p.mods.clone(), p.passed))
                .collect();
            players.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            (r.map_id, players)
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_default()
}

// What a saved score and its map results are compared by.
pub fn score_key(team1: Option<i32>, team2: Option<i32>, maps: &[MapResultRow]) -> String {
    let maps: Vec<_> = maps
        .iter()
        .map(|m| {
            let mut players: Vec<_> = m
                .players
                .iter()
                .map(|p| (p.user_id, p.score, p.mods.clone(), p.passed))
                .collect();
            players.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            (m.map_id, m.team1_score, m.team2_score, players)
        })
        .collect();
    serde_json::to_string(&(team1, team2, maps)).unwrap_or_default()
}
