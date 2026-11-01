//! A stage's statistics, counted from the scores the site saved: every team's and every
//! player's plays on each pool map, each measured against the field that played the same
//! map (its Z score: how many standard deviations above or below that map's mean), the
//! maps' picks, bans and best scores, and the standings.
//!
//! The qualifier decides the seeding, so its rules differ from a bracket stage's:
//! - a team's latest attempt at a map is the one that counts (a replay replaces it);
//! - a team (or player) without a score on a map whose lobby is over gets the Z a score
//!   of 0 would have against those who played it, without counting in that map's mean
//!   and deviation, so a no-show doesn't move everyone else's Z;
//! - teams are seeded by the sum of their Z scores over the pool.
//!
//! In a bracket stage every play counts (a team can play a map in two matches), and
//! teams and players are ranked by their average Z.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

// A saved player score on a pool map.
#[derive(Debug, Clone)]
pub struct ScoreIn {
    pub match_id: Uuid,
    pub position: i32,
    pub map_id: Uuid,
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub score: i64,
    pub accuracy: Option<f64>,
    pub mods: Option<String>,
    pub passed: bool,
}

// A match of the stage, in play order.
#[derive(Debug, Clone)]
pub struct MatchIn {
    pub id: Uuid,
    pub concluded: bool,
    // Qualifier lobbies: the teams signed up.
    pub teams: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DraftIn {
    pub match_id: Uuid,
    pub map_id: Uuid,
    pub team_id: Uuid,
    pub ban: bool,
}

pub struct Input<'a> {
    pub qualifier: bool,
    // The pool, in category and slot order.
    pub maps: &'a [Uuid],
    pub matches: &'a [MatchIn],
    pub scores: &'a [ScoreIn],
    pub drafts: &'a [DraftIn],
    // Every team's players.
    pub members: &'a HashMap<Uuid, Vec<Uuid>>,
}

// The plays on a map: how many, their mean and their (sample) standard deviation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Field {
    pub count: usize,
    pub mean: f64,
    pub sd: f64,
}

impl Field {
    fn of(values: &[f64]) -> Field {
        let count = values.len();
        if count == 0 {
            return Field {
                count,
                mean: 0.0,
                sd: 0.0,
            };
        }
        let mean = values.iter().sum::<f64>() / count as f64;
        let sd = if count < 2 {
            0.0
        } else {
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64).sqrt()
        };
        Field { count, mean, sd }
    }

    // How far `value` stands from the field, in deviations; 0 when the field can't say.
    fn z(&self, value: f64) -> f64 {
        if self.sd > 0.0 {
            (value - self.mean) / self.sd
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerPlay {
    pub user_id: Uuid,
    pub score: i64,
    pub accuracy: Option<f64>,
    pub mods: Option<String>,
    pub passed: bool,
    pub z: f64,
    // Among every player's play on the map, best first; level scores share it.
    pub rank: usize,
}

// A team's play on a map: its players' scores summed.
#[derive(Debug, Clone, Serialize)]
pub struct TeamPlay {
    pub match_id: Uuid,
    pub map_id: Uuid,
    pub team_id: Uuid,
    pub score: i64,
    // The players' average.
    pub accuracy: Option<f64>,
    pub z: f64,
    pub rank: usize,
    // Bracket matches: whether the team won the map (None for a tie).
    pub won: Option<bool>,
    // Bracket matches: whether the team picked it.
    pub picked: bool,
    // Best first.
    pub players: Vec<PlayerPlay>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BestTeam {
    pub team_id: Uuid,
    pub match_id: Uuid,
    pub score: i64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BestPlayer {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub match_id: Uuid,
    pub score: i64,
    pub accuracy: Option<f64>,
    pub mods: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapStats {
    pub map_id: Uuid,
    pub teams: Field,
    pub players: Field,
    pub avg_accuracy: Option<f64>,
    pub picks: usize,
    pub bans: usize,
    // Of the plays of a picked map, how many the team that picked it won.
    pub picker_wins: usize,
    pub picker_plays: usize,
    pub best_team: Option<BestTeam>,
    pub best_player: Option<BestPlayer>,
}

// A team's or a player's standing on one map.
#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub map_id: Uuid,
    // The play's Z; the average over a bracket stage's plays of the map; for a missing
    // qualifier map, the Z of a score of 0.
    pub z: f64,
    // The best score on it (none when missing).
    pub score: Option<i64>,
    pub accuracy: Option<f64>,
    pub rank: Option<usize>,
    pub plays: usize,
    pub missing: bool,
}

// The play furthest above its field.
#[derive(Debug, Clone, Serialize)]
pub struct Standout {
    pub map_id: Uuid,
    pub match_id: Uuid,
    pub z: f64,
    pub score: i64,
    pub rank: usize,
    // How many played the map.
    pub field: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    // The team's, or the player's.
    pub id: Uuid,
    pub team_id: Uuid,
    // Seed in the qualifier; None until there's anything to rank.
    pub rank: Option<usize>,
    pub z_sum: f64,
    pub z_avg: f64,
    pub plays: usize,
    // The maps counted in z_sum, missing ones included.
    pub maps_counted: usize,
    pub total_score: i64,
    pub avg_score: f64,
    pub avg_accuracy: Option<f64>,
    // Bracket stages: the maps their team won and lost.
    pub map_wins: usize,
    pub map_losses: usize,
    // Teams: their picks and bans. Players: the plays on which they outscored their
    // teammate.
    pub picks: usize,
    pub bans: usize,
    pub team_best: usize,
    // Qualifier: their lobby isn't over, so their missing maps don't count yet.
    pub pending: bool,
    pub best: Option<Standout>,
    pub per_map: Vec<Cell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    // Matches with a saved map, a ban or a pick.
    pub matches_played: usize,
    pub plays: Vec<TeamPlay>,
    pub maps: Vec<MapStats>,
    pub teams: Vec<Standing>,
    pub players: Vec<Standing>,
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / n as f64)
}

// Each value's place, best (highest) first; level values share it.
fn ranks(values: &[i64]) -> Vec<usize> {
    values
        .iter()
        .map(|v| values.iter().filter(|o| *o > v).count() + 1)
        .collect()
}

// An attempt: one (match, position) of one team.
struct Attempt<'a> {
    match_id: Uuid,
    position: i32,
    map_id: Uuid,
    team_id: Uuid,
    scores: Vec<&'a ScoreIn>,
}

pub fn compute(input: &Input) -> Stats {
    let pool: HashSet<Uuid> = input.maps.iter().copied().collect();
    let match_order: HashMap<Uuid, usize> = input
        .matches
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id, i))
        .collect();

    // Group the scores into attempts, in play order.
    let mut attempts: Vec<Attempt> = Vec::new();
    for s in input.scores.iter().filter(|s| pool.contains(&s.map_id)) {
        match attempts.iter_mut().find(|a| {
            a.match_id == s.match_id && a.position == s.position && a.team_id == s.team_id
        }) {
            Some(a) => a.scores.push(s),
            None => attempts.push(Attempt {
                match_id: s.match_id,
                position: s.position,
                map_id: s.map_id,
                team_id: s.team_id,
                scores: vec![s],
            }),
        }
    }
    let order = |a: &Attempt| {
        (
            match_order.get(&a.match_id).copied().unwrap_or(usize::MAX),
            a.position,
        )
    };
    attempts.sort_by_key(order);

    // The opponent's total on the same map of a bracket match, for who won it.
    let totals: HashMap<(Uuid, i32, Uuid), i64> = attempts
        .iter()
        .map(|a| {
            (
                (a.match_id, a.position, a.team_id),
                a.scores.iter().map(|s| s.score).sum(),
            )
        })
        .collect();

    // In the qualifier only a team's latest attempt at a map counts.
    let counted: Vec<&Attempt> = if input.qualifier {
        let mut latest: HashMap<(Uuid, Uuid), &Attempt> = HashMap::new();
        for a in &attempts {
            latest.insert((a.team_id, a.map_id), a);
        }
        attempts
            .iter()
            .filter(|a| {
                latest
                    .get(&(a.team_id, a.map_id))
                    .is_some_and(|l| std::ptr::eq(*l, *a))
            })
            .collect()
    } else {
        attempts.iter().collect()
    };

    let picked: HashSet<(Uuid, Uuid, Uuid)> = input
        .drafts
        .iter()
        .filter(|d| !d.ban)
        .map(|d| (d.match_id, d.map_id, d.team_id))
        .collect();

    // Each map's fields and every play's Z and place.
    let mut plays: Vec<TeamPlay> = Vec::new();
    let mut maps: Vec<MapStats> = Vec::new();
    for &map_id in input.maps {
        let on_map: Vec<&&Attempt> = counted.iter().filter(|a| a.map_id == map_id).collect();
        let team_scores: Vec<i64> = on_map
            .iter()
            .map(|a| a.scores.iter().map(|s| s.score).sum())
            .collect();
        let player_scores: Vec<&ScoreIn> = on_map.iter().flat_map(|a| a.scores.clone()).collect();
        let player_values: Vec<i64> = player_scores.iter().map(|s| s.score).collect();
        let teams = Field::of(&team_scores.iter().map(|v| *v as f64).collect::<Vec<_>>());
        let players = Field::of(&player_values.iter().map(|v| *v as f64).collect::<Vec<_>>());
        let team_ranks = ranks(&team_scores);
        let player_ranks: HashMap<*const ScoreIn, usize> = player_scores
            .iter()
            .zip(ranks(&player_values))
            .map(|(s, r)| (*s as *const ScoreIn, r))
            .collect();

        let mut best_team: Option<BestTeam> = None;
        let mut best_player: Option<BestPlayer> = None;
        let (mut picker_wins, mut picker_plays) = (0, 0);
        for (i, a) in on_map.iter().enumerate() {
            let score = team_scores[i];
            let opponent = totals
                .iter()
                .find(|((m, p, t), _)| *m == a.match_id && *p == a.position && *t != a.team_id)
                .map(|(_, v)| *v);
            let won = match (input.qualifier, opponent) {
                (false, Some(o)) if o != score => Some(score > o),
                _ => None,
            };
            let is_pick = picked.contains(&(a.match_id, map_id, a.team_id));
            if is_pick {
                picker_plays += 1;
                if won == Some(true) {
                    picker_wins += 1;
                }
            }
            let mut ps: Vec<PlayerPlay> = a
                .scores
                .iter()
                .map(|s| PlayerPlay {
                    user_id: s.user_id,
                    score: s.score,
                    accuracy: s.accuracy,
                    mods: s.mods.clone(),
                    passed: s.passed,
                    z: players.z(s.score as f64),
                    rank: player_ranks[&(*s as *const ScoreIn)],
                })
                .collect();
            ps.sort_by_key(|x| std::cmp::Reverse(x.score));
            let accuracy = mean(a.scores.iter().filter_map(|s| s.accuracy));
            if best_team.as_ref().is_none_or(|b| score > b.score) {
                best_team = Some(BestTeam {
                    team_id: a.team_id,
                    match_id: a.match_id,
                    score,
                    accuracy,
                });
            }
            for s in &a.scores {
                if best_player.as_ref().is_none_or(|b| s.score > b.score) {
                    best_player = Some(BestPlayer {
                        user_id: s.user_id,
                        team_id: s.team_id,
                        match_id: s.match_id,
                        score: s.score,
                        accuracy: s.accuracy,
                        mods: s.mods.clone(),
                    });
                }
            }
            plays.push(TeamPlay {
                match_id: a.match_id,
                map_id,
                team_id: a.team_id,
                score,
                accuracy,
                z: teams.z(score as f64),
                rank: team_ranks[i],
                won,
                picked: is_pick,
                players: ps,
            });
        }
        let marks = |ban: bool| {
            input
                .drafts
                .iter()
                .filter(|d| d.map_id == map_id && d.ban == ban)
                .count()
        };
        maps.push(MapStats {
            map_id,
            teams,
            players,
            avg_accuracy: mean(player_scores.iter().filter_map(|s| s.accuracy)),
            picks: marks(false),
            bans: marks(true),
            picker_wins,
            picker_plays,
            best_team,
            best_player,
        });
    }

    let matches_played = input
        .matches
        .iter()
        .filter(|m| {
            plays.iter().any(|p| p.match_id == m.id)
                || input.drafts.iter().any(|d| d.match_id == m.id)
        })
        .count();

    // The teams: the qualifier's are every team signed up for a lobby, plus any that
    // played; a bracket stage's are those that played.
    let mut team_ids: Vec<Uuid> = Vec::new();
    let mut add = |id: Uuid| {
        if !team_ids.contains(&id) {
            team_ids.push(id);
        }
    };
    if input.qualifier {
        for m in input.matches {
            m.teams.iter().copied().for_each(&mut add);
        }
    }
    plays.iter().for_each(|p| add(p.team_id));

    // A qualifier team's missing maps count once its lobby is over (or it has none left).
    let lobby_over = |team: Uuid| {
        input
            .matches
            .iter()
            .find(|m| m.teams.contains(&team))
            .is_none_or(|m| m.concluded)
    };
    let field_of = |map_id: Uuid| maps.iter().find(|m| m.map_id == map_id);

    let mut teams: Vec<Standing> = team_ids
        .iter()
        .map(|&team_id| {
            let own: Vec<&TeamPlay> = plays.iter().filter(|p| p.team_id == team_id).collect();
            let pending = input.qualifier && !lobby_over(team_id);
            let mut per_map = Vec::new();
            for &map_id in input.maps {
                let on: Vec<&&TeamPlay> = own.iter().filter(|p| p.map_id == map_id).collect();
                let field = field_of(map_id).map(|m| m.teams);
                if let Some(cell) = cell_of(
                    map_id,
                    &on.iter()
                        .map(|p| (p.z, p.score, p.accuracy, p.rank))
                        .collect::<Vec<_>>(),
                    field,
                    input.qualifier && !pending,
                ) {
                    per_map.push(cell);
                }
            }
            let draft = |ban: bool| {
                input
                    .drafts
                    .iter()
                    .filter(|d| d.team_id == team_id && d.ban == ban)
                    .count()
            };
            standing(
                team_id,
                team_id,
                &own.iter()
                    .map(|p| Played {
                        map_id: p.map_id,
                        match_id: p.match_id,
                        z: p.z,
                        score: p.score,
                        rank: p.rank,
                        accuracy: p.accuracy,
                        won: p.won,
                        team_best: false,
                    })
                    .collect::<Vec<_>>(),
                per_map,
                pending,
                draft(false),
                draft(true),
                &maps,
                |m| m.teams.count,
            )
        })
        .collect();

    // The players: every member of those teams.
    let mut players: Vec<Standing> = Vec::new();
    for &team_id in &team_ids {
        let pending = input.qualifier && !lobby_over(team_id);
        let own: Vec<&TeamPlay> = plays.iter().filter(|p| p.team_id == team_id).collect();
        let mut users: Vec<Uuid> = input.members.get(&team_id).cloned().unwrap_or_default();
        for p in &own {
            for pp in &p.players {
                if !users.contains(&pp.user_id) {
                    users.push(pp.user_id);
                }
            }
        }
        for user_id in users {
            let mine: Vec<Played> = own
                .iter()
                .filter_map(|p| {
                    let me = p.players.iter().find(|pp| pp.user_id == user_id)?;
                    Some(Played {
                        map_id: p.map_id,
                        match_id: p.match_id,
                        z: me.z,
                        score: me.score,
                        rank: me.rank,
                        accuracy: me.accuracy,
                        won: p.won,
                        // Level with their teammate isn't outscoring them.
                        team_best: p.players.len() > 1
                            && p.players[0].user_id == user_id
                            && p.players[0].score > p.players[1].score,
                    })
                })
                .collect();
            let mut per_map = Vec::new();
            for &map_id in input.maps {
                let on: Vec<(f64, i64, Option<f64>, usize)> = mine
                    .iter()
                    .filter(|p| p.map_id == map_id)
                    .map(|p| (p.z, p.score, p.accuracy, p.rank))
                    .collect();
                let field = field_of(map_id).map(|m| m.players);
                if let Some(cell) = cell_of(map_id, &on, field, input.qualifier && !pending) {
                    per_map.push(cell);
                }
            }
            players.push(standing(
                user_id,
                team_id,
                &mine,
                per_map,
                pending,
                0,
                0,
                &maps,
                |m| m.players.count,
            ));
        }
    }

    rank_standings(&mut teams, input.qualifier);
    rank_standings(&mut players, input.qualifier);

    Stats {
        matches_played,
        plays,
        maps,
        teams,
        players,
    }
}

struct Played {
    map_id: Uuid,
    match_id: Uuid,
    z: f64,
    score: i64,
    rank: usize,
    accuracy: Option<f64>,
    won: Option<bool>,
    team_best: bool,
}

// A standing on a map from its plays there (z, score, accuracy, rank); a missing map
// gets the Z of a 0 against the field when `penalize`, and nothing otherwise. A map
// nobody has played yet has no cell.
fn cell_of(
    map_id: Uuid,
    plays: &[(f64, i64, Option<f64>, usize)],
    field: Option<Field>,
    penalize: bool,
) -> Option<Cell> {
    if plays.is_empty() {
        let field = field.filter(|f| f.count > 0 && penalize)?;
        return Some(Cell {
            map_id,
            z: field.z(0.0),
            score: None,
            accuracy: None,
            rank: None,
            plays: 0,
            missing: true,
        });
    }
    Some(Cell {
        map_id,
        z: plays.iter().map(|p| p.0).sum::<f64>() / plays.len() as f64,
        score: plays.iter().map(|p| p.1).max(),
        accuracy: mean(plays.iter().filter_map(|p| p.2)),
        rank: plays.iter().map(|p| p.3).min(),
        plays: plays.len(),
        missing: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn standing(
    id: Uuid,
    team_id: Uuid,
    played: &[Played],
    per_map: Vec<Cell>,
    pending: bool,
    picks: usize,
    bans: usize,
    maps: &[MapStats],
    field_size: impl Fn(&MapStats) -> usize,
) -> Standing {
    let total_score: i64 = played.iter().map(|p| p.score).sum();
    let z_sum: f64 = per_map.iter().map(|c| c.z).sum();
    let best = played
        .iter()
        .max_by(|a, b| a.z.total_cmp(&b.z))
        .map(|p| Standout {
            map_id: p.map_id,
            match_id: p.match_id,
            z: p.z,
            score: p.score,
            rank: p.rank,
            field: maps
                .iter()
                .find(|m| m.map_id == p.map_id)
                .map_or(0, &field_size),
        });
    Standing {
        id,
        team_id,
        rank: None,
        z_sum,
        z_avg: mean(played.iter().map(|p| p.z)).unwrap_or(0.0),
        plays: played.len(),
        maps_counted: per_map.len(),
        total_score,
        avg_score: if played.is_empty() {
            0.0
        } else {
            total_score as f64 / played.len() as f64
        },
        avg_accuracy: mean(played.iter().filter_map(|p| p.accuracy)),
        map_wins: played.iter().filter(|p| p.won == Some(true)).count(),
        map_losses: played.iter().filter(|p| p.won == Some(false)).count(),
        picks,
        bans,
        team_best: played.iter().filter(|p| p.team_best).count(),
        pending,
        best,
        per_map,
    }
}

// The qualifier seeds by the sum of Z (missing maps included), a bracket stage ranks by
// the average Z of the plays; level values share a place. Standings with nothing to
// rank go last, unranked.
fn rank_standings(rows: &mut [Standing], qualifier: bool) {
    let value = |s: &Standing| if qualifier { s.z_sum } else { s.z_avg };
    rows.sort_by(|a, b| {
        (b.maps_counted > 0)
            .cmp(&(a.maps_counted > 0))
            .then(value(b).total_cmp(&value(a)))
            .then(b.total_score.cmp(&a.total_score))
    });
    let mut previous: Option<(f64, usize)> = None;
    for (i, row) in rows.iter_mut().enumerate() {
        if row.maps_counted == 0 {
            continue;
        }
        let v = value(row);
        row.rank = Some(match previous {
            Some((pv, rank)) if (pv - v).abs() < 1e-9 => rank,
            _ => i + 1,
        });
        previous = Some((v, row.rank.unwrap()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    // Team t (100 + t) has players 10t + 1 and 10t + 2.
    fn members(teams: &[u128]) -> HashMap<Uuid, Vec<Uuid>> {
        teams
            .iter()
            .map(|&t| (id(100 + t), vec![id(10 * t + 1), id(10 * t + 2)]))
            .collect()
    }

    fn play(m: u128, position: i32, map: u128, team: u128, scores: [i64; 2]) -> Vec<ScoreIn> {
        scores
            .iter()
            .enumerate()
            .map(|(i, &score)| ScoreIn {
                match_id: id(m),
                position,
                map_id: id(map),
                user_id: id(10 * team + 1 + i as u128),
                team_id: id(100 + team),
                score,
                accuracy: Some(0.9 + i as f64 / 100.0),
                mods: None,
                passed: true,
            })
            .collect()
    }

    fn lobby(m: u128, teams: &[u128], concluded: bool) -> MatchIn {
        MatchIn {
            id: id(m),
            concluded,
            teams: teams.iter().map(|&t| id(100 + t)).collect(),
        }
    }

    #[test]
    fn a_field_is_measured_in_sample_deviations() {
        let f = Field::of(&[800.0, 700.0, 600.0, 500.0, 400.0]);
        assert_eq!(f.mean, 600.0);
        assert!((f.sd - 158.113_883).abs() < 1e-5);
        assert!((f.z(800.0) - 1.264_911).abs() < 1e-5);
        // One play says nothing about the field.
        assert_eq!(Field::of(&[5.0]).z(9.0), 0.0);
    }

    #[test]
    fn a_missing_qualifier_map_scores_as_a_zero_without_moving_the_field() {
        let maps = [id(1), id(2)];
        let members = members(&[1, 2, 3]);
        let matches = [lobby(50, &[1, 2, 3], true)];
        let scores: Vec<ScoreIn> = [
            play(50, 1, 1, 1, [500, 300]),
            play(50, 1, 1, 2, [400, 200]),
            play(50, 1, 1, 3, [300, 100]),
            play(50, 2, 2, 1, [500, 500]),
            play(50, 2, 2, 2, [300, 300]),
        ]
        .concat();
        let stats = compute(&Input {
            qualifier: true,
            maps: &maps,
            matches: &matches,
            scores: &scores,
            drafts: &[],
            members: &members,
        });
        let map2 = stats.maps.iter().find(|m| m.map_id == id(2)).unwrap();
        // Only the two teams that played map 2 make its field.
        assert_eq!(map2.teams.count, 2);
        assert_eq!(map2.teams.mean, 800.0);
        let team3 = stats.teams.iter().find(|t| t.id == id(103)).unwrap();
        let missing = team3.per_map.iter().find(|c| c.map_id == id(2)).unwrap();
        assert!(missing.missing);
        assert!((missing.z - map2.teams.z(0.0)).abs() < 1e-9);
        assert_eq!(team3.maps_counted, 2);
        // Seeded by the sum: team 1 won both maps.
        let seeds: Vec<(Uuid, Option<usize>)> =
            stats.teams.iter().map(|t| (t.id, t.rank)).collect();
        assert_eq!(
            seeds,
            vec![(id(101), Some(1)), (id(102), Some(2)), (id(103), Some(3))]
        );
    }

    #[test]
    fn a_lobby_still_being_played_isnt_penalized_yet() {
        let maps = [id(1), id(2)];
        let members = members(&[1, 2, 3]);
        let matches = [lobby(50, &[1, 2], true), lobby(51, &[3], false)];
        let scores: Vec<ScoreIn> = [
            play(50, 1, 1, 1, [500, 300]),
            play(50, 1, 1, 2, [400, 200]),
            play(50, 2, 2, 1, [500, 500]),
            play(50, 2, 2, 2, [300, 300]),
            play(51, 1, 1, 3, [300, 100]),
        ]
        .concat();
        let stats = compute(&Input {
            qualifier: true,
            maps: &maps,
            matches: &matches,
            scores: &scores,
            drafts: &[],
            members: &members,
        });
        let team3 = stats.teams.iter().find(|t| t.id == id(103)).unwrap();
        assert!(team3.pending);
        assert_eq!(team3.maps_counted, 1);
        assert!(team3.per_map.iter().all(|c| !c.missing));
    }

    #[test]
    fn a_replay_replaces_the_earlier_attempt_in_the_qualifier() {
        let maps = [id(1)];
        let members = members(&[1, 2]);
        let matches = [lobby(50, &[1, 2], true)];
        let scores: Vec<ScoreIn> = [
            play(50, 1, 1, 1, [900, 900]),
            play(50, 2, 1, 1, [100, 100]),
            play(50, 2, 1, 2, [400, 400]),
        ]
        .concat();
        let stats = compute(&Input {
            qualifier: true,
            maps: &maps,
            matches: &matches,
            scores: &scores,
            drafts: &[],
            members: &members,
        });
        let team1: Vec<i64> = stats
            .plays
            .iter()
            .filter(|p| p.team_id == id(101))
            .map(|p| p.score)
            .collect();
        assert_eq!(team1, vec![200]);
        assert_eq!(stats.teams[0].id, id(102));
    }

    #[test]
    fn a_bracket_stage_counts_every_play_with_its_winner_and_picker() {
        let maps = [id(1), id(2)];
        let members = members(&[1, 2]);
        let matches = [MatchIn {
            id: id(60),
            concluded: true,
            teams: vec![],
        }];
        let scores: Vec<ScoreIn> = [
            play(60, 1, 1, 1, [600, 500]),
            play(60, 1, 1, 2, [400, 300]),
            play(60, 2, 2, 1, [300, 300]),
            play(60, 2, 2, 2, [500, 500]),
        ]
        .concat();
        let drafts = [
            DraftIn {
                match_id: id(60),
                map_id: id(1),
                team_id: id(101),
                ban: false,
            },
            DraftIn {
                match_id: id(60),
                map_id: id(2),
                team_id: id(101),
                ban: false,
            },
            DraftIn {
                match_id: id(60),
                map_id: id(1),
                team_id: id(102),
                ban: true,
            },
        ];
        let stats = compute(&Input {
            qualifier: false,
            maps: &maps,
            matches: &matches,
            scores: &scores,
            drafts: &drafts,
            members: &members,
        });
        let map1 = &stats.maps[0];
        assert_eq!((map1.picks, map1.bans), (1, 1));
        assert_eq!((map1.picker_plays, map1.picker_wins), (1, 1));
        assert_eq!(map1.best_player.as_ref().unwrap().score, 600);
        let team1 = stats.teams.iter().find(|t| t.id == id(101)).unwrap();
        assert_eq!((team1.map_wins, team1.map_losses), (1, 1));
        assert_eq!((team1.picks, team1.bans), (2, 0));
        // Player 11 outscored their teammate on map 1 only.
        let p11 = stats.players.iter().find(|p| p.id == id(11)).unwrap();
        assert_eq!(p11.team_best, 1);
        assert_eq!(stats.matches_played, 1);
    }

    #[test]
    fn level_values_share_a_place() {
        assert_eq!(ranks(&[5, 9, 5, 1]), vec![2, 1, 2, 4]);
    }
}
