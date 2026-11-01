//! A match's finished maps and score, counted from its rooms: each finished pool map
//! goes to the team with the higher total, unless the referee left it out ("Sayma"),
//! plus the maps they awarded by hand. The server counts it, saves it and shows it to
//! every page, so no page has to.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

use super::{
    commands::{is_freemod_category, is_tiebreaker_category},
    irc::same_name,
    room::{split_mods, MapResult, MapScore, SlotTeam},
};
use crate::{
    db::{
        bracket::{BanRow, ManualResultRow, MapPlayerScore, MapResultRow, PickRow},
        mappool::{Category, PublicPoolMap, PublicSlot},
        stats::QualifierMapRow,
    },
    draft::{next_turn, DraftOrder, StepKind, Turn},
    osu_api::matches::MatchGame,
};

#[derive(Debug, Clone)]
pub struct PoolEntry {
    pub map_id: Uuid,
    pub beatmap_id: i64,
    // The map's own mods, as acronyms.
    pub mods: String,
    pub mode: String,
    pub freemod: bool,
    pub tiebreaker: bool,
}

// The pool in category and slot order.
pub fn pool_entries(
    categories: &[Category],
    slots: &[PublicSlot],
    maps: &[PublicPoolMap],
) -> Vec<PoolEntry> {
    let mut entries = Vec::new();
    for category in categories {
        let mut own: Vec<&PublicSlot> = slots
            .iter()
            .filter(|s| s.category_id == category.id)
            .collect();
        own.sort_by_key(|s| s.position);
        for slot in own {
            let Some(map) = maps.iter().find(|m| m.slot_id == Some(slot.id)) else {
                continue;
            };
            entries.push(PoolEntry {
                map_id: map.id,
                beatmap_id: map.beatmap_id,
                mods: map.mods.clone(),
                mode: map.mode.clone(),
                freemod: is_freemod_category(&category.name),
                tiebreaker: is_tiebreaker_category(&category.name),
            });
        }
    }
    entries
}

// Mods compared without NoFail, Nightcore as the DoubleTime it is, in a fixed order.
fn mod_key(acronyms: &str) -> String {
    let mut mods: Vec<String> = split_mods(acronyms)
        .into_iter()
        .filter(|m| m != "NF")
        .map(|m| if m == "NC" { "DT".into() } else { m })
        .collect();
    mods.sort();
    mods.concat()
}

// The pool entry a finished map was: the one with its beatmap. When the pool has that
// beatmap more than once, the one whose mods it was played with, then the one a team
// picked, then the first.
pub fn entry_of<'a>(
    entries: &'a [PoolEntry],
    beatmap_id: Option<i64>,
    room_mods: &str,
    freemod: bool,
    picked: &HashSet<Uuid>,
) -> Option<&'a PoolEntry> {
    let candidates: Vec<&PoolEntry> = entries
        .iter()
        .filter(|e| Some(e.beatmap_id) == beatmap_id)
        .collect();
    if candidates.len() <= 1 {
        return candidates.first().copied();
    }
    let played = mod_key(room_mods);
    let by_mods: Vec<&PoolEntry> = candidates
        .iter()
        .copied()
        .filter(|e| {
            if freemod {
                e.freemod
            } else {
                !e.freemod && mod_key(&e.mods) == played
            }
        })
        .collect();
    let pool = if by_mods.is_empty() {
        candidates
    } else {
        by_mods
    };
    pool.iter()
        .copied()
        .find(|e| picked.contains(&e.map_id))
        .or(pool.first().copied())
}

// A player of the match, with their team and, in a bracket match, where they belong.
#[derive(Debug, Clone)]
pub struct RosterPlayer {
    pub user_id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub team_id: Uuid,
    // Bracket matches: the colour the player's team plays as.
    pub color: Option<SlotTeam>,
    // Bracket matches: the room slot the player belongs in.
    pub home: Option<i32>,
}

// The roster player a score is by: by osu! id when it's known, by name otherwise.
pub fn scorer<'a>(score: &MapScore, roster: &'a [RosterPlayer]) -> Option<&'a RosterPlayer> {
    roster.iter().find(|p| match score.user_id {
        Some(id) => p.osu_id == id,
        None => same_name(&p.username, &score.username),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TeamTotal {
    pub team_id: Uuid,
    pub score: i64,
}

// Each team's summed score on a map, best first. Players not on the roster don't count.
pub fn team_totals(result: &MapResult, roster: &[RosterPlayer]) -> Vec<TeamTotal> {
    let mut totals: Vec<TeamTotal> = Vec::new();
    for s in &result.scores {
        if let Some(r) = scorer(s, roster) {
            match totals.iter_mut().find(|t| t.team_id == r.team_id) {
                Some(t) => t.score += s.score,
                None => totals.push(TeamTotal {
                    team_id: r.team_id,
                    score: s.score,
                }),
            }
        }
    }
    totals.sort_by_key(|t| std::cmp::Reverse(t.score));
    totals
}

// A finished map of one of the match's rooms.
#[derive(Debug, Clone)]
pub struct Finished {
    // "<source>:<room>:<n>", the nth finished map of the room; "Sayma" is keyed by it.
    pub key: String,
    pub mp_id: i64,
    pub result: MapResult,
}

// osu!'s record of a room as finished maps; games nobody finished are left out. osu!
// doesn't say whether freemod was on, so a map counts as freemod when a player had mods
// the room didn't give everyone (NoFail aside).
pub fn osu_results(games: &[MatchGame]) -> Vec<MapResult> {
    games
        .iter()
        .filter(|g| !g.scores.is_empty())
        .map(|g| {
            let room_mods = g.mods.concat();
            let room: Vec<String> = split_mods(&room_mods);
            let scores: Vec<MapScore> = g
                .scores
                .iter()
                .map(|sc| {
                    let mut mods = room.clone();
                    mods.extend(
                        split_mods(&sc.mods.concat())
                            .into_iter()
                            .filter(|m| !room.contains(m)),
                    );
                    MapScore {
                        username: String::new(),
                        user_id: Some(sc.user_id),
                        score: sc.score,
                        passed: sc.passed,
                        mods: Some(mods.concat()),
                    }
                })
                .collect();
            let freemod = scores.iter().any(|sc| {
                split_mods(sc.mods.as_deref().unwrap_or_default())
                    .iter()
                    .any(|m| !room.contains(m) && m != "NF")
            });
            MapResult {
                beatmap_id: g.beatmap_id,
                beatmap_name: None,
                room_mods,
                freemod,
                scores,
                teams: HashMap::new(),
            }
        })
        .collect()
}

// Whether two finished maps are the same play: the same beatmap and a score in common.
fn same_play(a: &MapResult, b: &MapResult) -> bool {
    if a.beatmap_id != b.beatmap_id {
        return false;
    }
    let scores: HashSet<i64> = a
        .scores
        .iter()
        .filter(|s| s.score > 0)
        .map(|s| s.score)
        .collect();
    b.scores
        .iter()
        .any(|s| s.score > 0 && scores.contains(&s.score))
}

// A room's finished maps. The chat's, unless the referee synced the results from osu!:
// then every map osu! recorded comes from there instead, followed by the ones the chat
// saw finish after the last of them. When the chat has none of osu!'s maps, they're told
// apart by count.
pub fn room_results(mp_id: i64, chat: &[MapResult], games: Option<&[MatchGame]>) -> Vec<Finished> {
    let from_chat = chat.iter().enumerate().map(|(n, result)| Finished {
        key: format!("chat:{mp_id}:{n}"),
        mp_id,
        result: result.clone(),
    });
    let Some(games) = games else {
        return from_chat.collect();
    };
    let from_osu: Vec<Finished> = osu_results(games)
        .into_iter()
        .enumerate()
        .map(|(n, result)| Finished {
            key: format!("osu:{mp_id}:{n}"),
            mp_id,
            result,
        })
        .collect();
    let mut last: Option<usize> = None;
    for (i, c) in chat.iter().enumerate() {
        if from_osu.iter().any(|o| same_play(&o.result, c)) {
            last = Some(i);
        }
    }
    let skip = match last {
        Some(i) => i + 1,
        None => from_osu.len(),
    };
    from_osu.into_iter().chain(from_chat.skip(skip)).collect()
}

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

// Where a bracket match stands against its best of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Race {
    Open,
    MatchPoint,
    // Both teams are a map away, so the tiebreaker is played.
    Tiebreaker,
    Won,
    Over,
}

pub fn race_of(best_of: i32, team1: i32, team2: i32) -> Race {
    let need = (best_of + 1) / 2;
    if team1 > need || team2 > need || (team1 >= need && team2 >= need) {
        return Race::Over;
    }
    if team1 == need || team2 == need {
        return Race::Won;
    }
    if need > 1 && team1 == need - 1 && team2 == need - 1 {
        return Race::Tiebreaker;
    }
    if team1 == need - 1 || team2 == need - 1 {
        Race::MatchPoint
    } else {
        Race::Open
    }
}

// Whose turn it is to ban or pick, and which: the stage's order read against the bans
// and picks so far, in the order they were marked (draft::next_turn).
pub fn turn_of(
    order: &DraftOrder,
    bans: &[BanRow],
    picks: &[PickRow],
    teams: [Uuid; 2],
    chosen: Option<Uuid>,
) -> Option<Turn> {
    let mut marks: Vec<_> = bans
        .iter()
        .map(|b| (b.created_at, b.team_id))
        .chain(picks.iter().map(|p| (p.created_at, p.team_id)))
        .collect();
    marks.sort_by_key(|m| m.0);
    let marks: Vec<Uuid> = marks.into_iter().map(|(_, team)| team).collect();
    next_turn(order, &marks, teams, chosen)
}

// The line sent to the room: "Skor: Team A 2 - 1 Team B | Sıradaki pick: Team B" (or
// "Sıradaki ban").
pub fn score_message(names: [&str; 2], score: [i32; 2], next: Option<(StepKind, &str)>) -> String {
    let line = format!(
        "Skor: {} {} - {} {}",
        names[0], score[0], score[1], names[1]
    );
    match next {
        Some((StepKind::Pick, team)) => format!("{line} | Sıradaki pick: {team}"),
        Some((StepKind::Ban, team)) => format!("{line} | Sıradaki ban: {team}"),
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u128, beatmap: i64, mods: &str, freemod: bool) -> PoolEntry {
        PoolEntry {
            map_id: Uuid::from_u128(id),
            beatmap_id: beatmap,
            mods: mods.into(),
            mode: "osu".into(),
            freemod,
            tiebreaker: false,
        }
    }

    fn score(name: &str, value: i64) -> MapScore {
        MapScore {
            username: name.into(),
            user_id: None,
            score: value,
            passed: true,
            mods: Some(String::new()),
        }
    }

    fn result(beatmap: i64, scores: Vec<MapScore>) -> MapResult {
        MapResult {
            beatmap_id: Some(beatmap),
            beatmap_name: None,
            room_mods: String::new(),
            freemod: false,
            scores,
            teams: HashMap::new(),
        }
    }

    #[test]
    fn tells_a_beatmap_apart_by_mods() {
        let entries = [
            entry(1, 5, "HD", false),
            entry(2, 5, "", true),
            entry(3, 5, "DT", false),
        ];
        let none = HashSet::new();
        assert_eq!(
            entry_of(&entries, Some(5), "NCNF", false, &none)
                .unwrap()
                .map_id,
            Uuid::from_u128(3)
        );
        assert_eq!(
            entry_of(&entries, Some(5), "", true, &none).unwrap().map_id,
            Uuid::from_u128(2)
        );
        assert!(entry_of(&entries, Some(6), "", false, &none).is_none());
    }

    #[test]
    fn saves_a_lobbys_pool_maps_but_not_the_ones_left_out() {
        let team = Uuid::from_u128(10);
        let roster = [RosterPlayer {
            user_id: Uuid::from_u128(1),
            osu_id: 1,
            username: "a".into(),
            team_id: team,
            color: None,
            home: None,
        }];
        let finished: Vec<Finished> = [
            result(5, vec![score("a", 100), score("stranger", 999)]),
            result(9, vec![score("a", 200)]),
            result(6, vec![score("a", 300)]),
            result(5, vec![score("a", 400)]),
        ]
        .into_iter()
        .enumerate()
        .map(|(n, result)| Finished {
            key: format!("chat:1:{n}"),
            mp_id: 1,
            result,
        })
        .collect();
        let entries = [entry(1, 5, "", false), entry(2, 6, "", false)];
        let ignored = HashSet::from(["chat:1:2".to_string()]);
        let rows = lobby_rows(&finished, &entries, &roster, &ignored);
        // The warmup (9) isn't in the pool and map 6 was left out; both attempts at 5 stay.
        let saved: Vec<(Option<Uuid>, Vec<i64>)> = rows
            .iter()
            .map(|r| (r.map_id, r.players.iter().map(|p| p.score).collect()))
            .collect();
        assert_eq!(
            saved,
            vec![
                (Some(Uuid::from_u128(1)), vec![100]),
                (Some(Uuid::from_u128(1)), vec![400]),
            ]
        );
    }

    #[test]
    fn counts_the_score() {
        let (t1, t2) = (Uuid::from_u128(10), Uuid::from_u128(20));
        let roster = [
            RosterPlayer {
                user_id: Uuid::from_u128(1),
                osu_id: 1,
                username: "a".into(),
                team_id: t1,
                color: None,
                home: None,
            },
            RosterPlayer {
                user_id: Uuid::from_u128(2),
                osu_id: 2,
                username: "b".into(),
                team_id: t2,
                color: None,
                home: None,
            },
        ];
        let entries = [entry(100, 5, "", false), entry(200, 6, "", false)];
        let finished = room_results(
            1,
            &[
                result(5, vec![score("a", 10), score("b", 20)]),
                result(6, vec![score("A", 30), score("b", 20)]),
                result(7, vec![score("a", 30)]),
            ],
            None,
        );
        let ignored = HashSet::new();
        let input = TallyInput {
            finished: &finished,
            entries: &entries,
            picks: &[],
            roster: &roster,
            ignored: &ignored,
            manual: &[],
            team1_id: t1,
            team2_id: t2,
        };
        let t = tally(&input);
        assert_eq!((t.team1, t.team2), (1, 1));
        assert_eq!(t.maps.len(), 2);
        assert_eq!((t.maps[0].team1_score, t.maps[0].team2_score), (10, 20));
        // "Sayma" leaves a map out.
        let ignored: HashSet<String> = ["chat:1:1".to_string()].into();
        let t = tally(&TallyInput {
            ignored: &ignored,
            ..input
        });
        assert_eq!((t.team1, t.team2), (0, 1));
    }

    #[test]
    fn osu_replaces_the_chat_up_to_its_last_map() {
        let game = |beatmap: i64, value: i64| MatchGame {
            beatmap_id: Some(beatmap),
            mods: vec![],
            end_time: Some(String::new()),
            scores: vec![crate::osu_api::matches::GameScore {
                user_id: 1,
                score: value,
                passed: true,
                accuracy: None,
                mods: vec!["HD".into()],
            }],
        };
        let chat = [
            result(5, vec![score("a", 10)]),
            result(6, vec![score("a", 11)]),
            result(7, vec![score("a", 12)]),
        ];
        let games = [game(5, 10), game(6, 11)];
        let keys: Vec<String> = room_results(9, &chat, Some(&games))
            .into_iter()
            .map(|f| f.key)
            .collect();
        assert_eq!(keys, ["osu:9:0", "osu:9:1", "chat:9:2"]);
        assert!(osu_results(&games)[0].freemod);
    }

    #[test]
    fn races_and_picks() {
        assert_eq!(race_of(9, 4, 4), Race::Tiebreaker);
        assert_eq!(race_of(9, 5, 3), Race::Won);
        assert_eq!(race_of(9, 4, 2), Race::MatchPoint);
        assert_eq!(race_of(9, 6, 0), Race::Over);
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let at = |s: i64| time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(s);
        let order = DraftOrder::parse("p1-p2-b2-b1-p1-*").unwrap();
        // Marked across both lists, in time order: b picked first, then a, then a banned.
        let picks = [
            PickRow {
                map_id: Uuid::nil(),
                team_id: b,
                created_at: at(0),
            },
            PickRow {
                map_id: Uuid::nil(),
                team_id: a,
                created_at: at(10),
            },
        ];
        let bans = [BanRow {
            map_id: Uuid::nil(),
            team_id: a,
            created_at: at(20),
        }];
        assert_eq!(
            turn_of(&order, &bans, &picks, [a, b], None),
            Some(Turn {
                kind: StepKind::Ban,
                team_id: Some(b)
            })
        );
        assert_eq!(
            score_message(["A", "B"], [2, 1], Some((StepKind::Pick, "B"))),
            "Skor: A 2 - 1 B | Sıradaki pick: B"
        );
    }
}
