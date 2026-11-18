use super::count::player_scores;
use super::*;
use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::bancho::room::{MapResult, MapScore};
use crate::{
    db::bracket::{BanRow, PickRow},
    draft::{DraftOrder, StepKind, Turn},
    osu_api::matches::MatchGame,
};

fn entry(id: u128, beatmap: i64, mods: &str, freemod: bool) -> PoolEntry {
    PoolEntry {
        map_id: Uuid::from_u128(id),
        beatmap_id: beatmap,
        mods: mods.into(),
        mode: "osu".into(),
        freemod,
        tiebreaker: false,
        label: format!("M{id}"),
        category: "M".into(),
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
fn a_failed_score_counts_like_any_other() {
    let (t1, t2) = (Uuid::from_u128(10), Uuid::from_u128(20));
    let player = |id: u128, name: &str, team| RosterPlayer {
        user_id: Uuid::from_u128(id),
        osu_id: id as i64,
        username: name.into(),
        team_id: team,
        color: None,
        home: None,
    };
    let roster = [
        player(1, "a1", t1),
        player(2, "a2", t1),
        player(3, "b1", t2),
        player(4, "b2", t2),
    ];
    // a1 failed without NoFail: 300k still counts, so team 1 has 900k against 800k.
    let failed = MapScore {
        passed: false,
        ..score("a1", 300_000)
    };
    let map = result(
        5,
        vec![
            failed,
            score("a2", 600_000),
            score("b1", 400_000),
            score("b2", 400_000),
        ],
    );
    let entries = [entry(100, 5, "", false)];
    let finished = room_results(1, std::slice::from_ref(&map), None);
    let ignored = HashSet::new();
    let t = tally(&TallyInput {
        finished: &finished,
        entries: &entries,
        picks: &[],
        roster: &roster,
        ignored: &ignored,
        manual: &[],
        team1_id: t1,
        team2_id: t2,
    });
    assert_eq!((t.team1, t.team2), (1, 0));
    assert_eq!(
        (t.maps[0].team1_score, t.maps[0].team2_score),
        (900_000, 800_000)
    );
    // It's saved with the player's score, marked failed.
    let saved = player_scores(&map, &roster);
    let a1 = saved
        .iter()
        .find(|p| p.user_id == Uuid::from_u128(1))
        .unwrap();
    assert_eq!((a1.score, a1.passed), (300_000, false));
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
    assert_eq!(standing_of(9, 4, 4), Standing::Tiebreaker);
    assert_eq!(standing_of(9, 5, 3), Standing::Won);
    assert_eq!(standing_of(9, 4, 2), Standing::MatchPoint);
    assert_eq!(standing_of(9, 6, 0), Standing::Over);
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
