//! The maps finished in a match's rooms, from the chat and from osu!'s record, and who
//! scored what on them.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

use crate::bancho::{
    names::same_name,
    room::{split_mods, MapResult, MapScore, SlotTeam},
};
use crate::osu_api::matches::MatchGame;

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
