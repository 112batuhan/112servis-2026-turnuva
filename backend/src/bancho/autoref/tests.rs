use super::*;
use crate::bancho::room::{RoomSlot, SlotStatus, SlotTeam, MATCH_STARTED};

const A: Uuid = Uuid::from_u128(1);
const B: Uuid = Uuid::from_u128(2);

fn at(secs: i64) -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + Duration::hours(1000) + Duration::seconds(secs)
}

fn player(name: &str, osu_id: i64, team: Uuid) -> RosterPlayer {
    RosterPlayer {
        user_id: Uuid::from_u128(osu_id as u128 + 100),
        osu_id,
        username: name.into(),
        team_id: team,
        color: None,
        home: None,
    }
}

fn entry(id: u128, label: &str, category: &str, freemod: bool, tb: bool) -> PoolEntry {
    PoolEntry {
        map_id: Uuid::from_u128(id),
        beatmap_id: id as i64,
        mods: String::new(),
        mode: "osu".into(),
        freemod,
        tiebreaker: tb,
        label: label.into(),
        category: category.into(),
    }
}

fn slot(n: i32, name: &str, osu_id: i64, status: SlotStatus, mods: &str) -> RoomSlot {
    RoomSlot {
        slot: n,
        username: name.into(),
        user_id: Some(osu_id),
        team: Some(SlotTeam::Blue),
        status,
        host: false,
        mods: mods.into(),
        mods_known: true,
    }
}

struct World {
    teams: Vec<MatchTeam>,
    roster: Vec<RosterPlayer>,
    entries: Vec<PoolEntry>,
    bans: Vec<BanRow>,
    picks: Vec<PickRow>,
    played: HashSet<Uuid>,
    room: RoomState,
    turn: Option<Turn>,
    score: Option<(i32, i32)>,
    bracket: bool,
}

impl World {
    fn bracket() -> Self {
        World {
            teams: vec![
                MatchTeam {
                    id: A,
                    name: "Alfa".into(),
                },
                MatchTeam {
                    id: B,
                    name: "Beta".into(),
                },
            ],
            roster: vec![
                player("a1", 1, A),
                player("a2", 2, A),
                player("b1", 3, B),
                player("b2", 4, B),
            ],
            entries: vec![
                entry(10, "NM1", "NM", false, false),
                entry(11, "NM2", "NM", false, false),
                entry(20, "HD1", "HD", false, false),
                entry(21, "HD2", "HD", false, false),
                entry(30, "FM1", "FM", true, false),
                entry(90, "TB", "TB", true, true),
            ],
            bans: Vec::new(),
            picks: Vec::new(),
            played: HashSet::new(),
            room: RoomState::default(),
            turn: None,
            score: Some((0, 0)),
            bracket: true,
        }
    }

    fn everyone_in(&mut self, status: SlotStatus) {
        self.room.slots = self
            .roster
            .iter()
            .enumerate()
            .map(|(i, p)| slot(i as i32 + 1, &p.username, p.osu_id, status, ""))
            .collect();
    }

    fn snap(&self, now: OffsetDateTime) -> Snapshot<'_> {
        Snapshot {
            now,
            bracket: self.bracket,
            starts_at: Some(at(0)),
            room_added_at: at(-600),
            teams: &self.teams,
            roster: &self.roster,
            entries: &self.entries,
            bans: &self.bans,
            picks: &self.picks,
            turn: self.turn,
            score: self.score,
            best_of: Some(5),
            played: &self.played,
            finished: &self.played,
            room: &self.room,
            no_fail: true,
            first_step_team: 1,
        }
    }
}

fn sent(out: &[Action]) -> Vec<&str> {
    out.iter()
        .filter_map(|a| match a {
            Action::Send(t) => Some(t.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn parses_rolls() {
    assert_eq!(
        parse_roll("Some Guy rolls 57 point(s)"),
        Some(("Some Guy", 57))
    );
    assert_eq!(parse_roll("x rolls point(s)"), None);
}

#[test]
fn waits_for_everyone_then_forfeits_the_missing_team() {
    let mut w = World::bracket();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    // Before the time, nothing.
    w.room.slots = vec![slot(1, "a1", 1, SlotStatus::NotReady, "")];
    ar.tick(&w.snap(at(-60)), &mut out);
    assert!(out.is_empty());
    // At the time, the missing are invited again, once, and told they have ten minutes.
    ar.tick(&w.snap(at(1)), &mut out);
    assert!(sent(&out).contains(&"!mp invite #2"));
    assert!(sent(&out)
        .iter()
        .any(|t| t.contains("10 dakika bekleniyor")));
    out.clear();
    ar.tick(&w.snap(at(2)), &mut out);
    assert!(out.is_empty());
    // A reminder every minute.
    ar.tick(&w.snap(at(61)), &mut out);
    assert_eq!(sent(&out), ["Eksik oyuncular için 9 dakika kaldı."]);
    out.clear();
    // Team A completes; B doesn't by the cutoff, which is ten seconds after the end the
    // room was told.
    w.room
        .slots
        .push(slot(2, "a2", 2, SlotStatus::NotReady, ""));
    w.room
        .slots
        .push(slot(3, "b1", 3, SlotStatus::NotReady, ""));
    ar.tick(&w.snap(at(591)), &mut out);
    assert_eq!(sent(&out), ["Eksik oyuncular için 10 saniye kaldı."]);
    ar.tick(&w.snap(at(605)), &mut out);
    assert!(!out.contains(&Action::Forfeit(2)));
    ar.tick(&w.snap(at(611)), &mut out);
    assert!(out.contains(&Action::Forfeit(2)));
    assert_eq!(ar.phase, Phase::Done);
}

#[test]
fn nobody_complete_hands_over() {
    let mut w = World::bracket();
    w.room.slots = vec![slot(1, "a1", 1, SlotStatus::NotReady, "")];
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    // Called late (the bot was away): the room still gets its ten minutes from then.
    ar.tick(&w.snap(at(700)), &mut out);
    ar.tick(&w.snap(at(1290)), &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Handover(_))));
    ar.tick(&w.snap(at(1310)), &mut out);
    assert!(out.iter().any(|a| matches!(a, Action::Handover(_))));
}

#[test]
fn qualifier_removes_incomplete_teams_and_opens_the_first_map() {
    let mut w = World::bracket();
    w.bracket = false;
    w.score = None;
    w.room.slots = vec![
        slot(1, "a1", 1, SlotStatus::NotReady, ""),
        slot(2, "a2", 2, SlotStatus::NotReady, ""),
        slot(3, "b1", 3, SlotStatus::NotReady, ""),
    ];
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(1)), &mut out);
    ar.tick(&w.snap(at(591)), &mut out);
    ar.tick(&w.snap(at(611)), &mut out);
    assert!(out.contains(&Action::RemoveTeams(vec![B])));
    assert!(sent(&out).contains(&"!mp kick #3"));
    assert!(sent(&out).contains(&"!mp map 10 0"));
    assert!(matches!(ar.phase, Phase::Ready { .. }));
}

#[test]
fn early_start_when_everyone_is_ready() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::Ready);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(-120)), &mut out);
    assert!(sent(&out).contains(&"!mp host #1"));
    assert!(matches!(ar.phase, Phase::Warmup { team: 0, .. }));
}

#[test]
fn warmups_then_roll_then_first_turn() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    // The second team skips ahead of its turn; the first plays one.
    ar.player(&w.snap(at(5)), "b2", "-warmup-gec", true, &mut out);
    w.room.beatmap_id = Some(555);
    w.room.playing = true;
    w.room.map_started_at = Some(at(10));
    ar.bancho(&w.snap(at(10)), MATCH_STARTED, &mut out);
    w.room.playing = false;
    out.clear();
    ar.bancho(&w.snap(at(100)), "The match has finished!", &mut out);
    assert!(matches!(ar.phase, Phase::Roll { .. }));
    assert!(sent(&out).contains(&"!mp clearhost"));
    // A roll over 100 doesn't count; the first fair ones decide.
    ar.bancho(&w.snap(at(101)), "a1 rolls 1000 point(s)", &mut out);
    ar.bancho(&w.snap(at(102)), "a1 rolls 20 point(s)", &mut out);
    ar.bancho(&w.snap(at(103)), "a2 rolls 99 point(s)", &mut out);
    out.clear();
    ar.bancho(&w.snap(at(104)), "b1 rolls 70 point(s)", &mut out);
    assert!(out.contains(&Action::FirstTurn(B)));
    assert_eq!(ar.phase, Phase::Next);
}

#[test]
fn countdowns_remind_and_keep_their_word() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    // A pick's two minutes: no !mp timer, the bot counts.
    ar.enter_ready(&w.snap(at(0)), &w.entries[0].clone(), &mut out);
    let opening = "Sıradaki map: NM1. Herkes hazır olduğunda ya da 2 dakika sonra map başlayacak.";
    assert!(sent(&out).contains(&opening));
    assert!(!sent(&out).iter().any(|t| t.starts_with("!mp timer")));
    ar.said(opening, at(0));
    out.clear();
    for t in [30, 59] {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    assert!(out.is_empty());
    ar.tick(&w.snap(at(60)), &mut out);
    assert_eq!(sent(&out), ["Map 1 dakika sonra başlayacak."]);
    // That line only went out at 66: the end moves to 66 + 60.
    ar.said("Map 1 dakika sonra başlayacak.", at(66));
    out.clear();
    ar.tick(&w.snap(at(90)), &mut out);
    assert!(out.is_empty());
    ar.tick(&w.snap(at(96)), &mut out);
    assert_eq!(sent(&out), ["Map 30 saniye sonra başlayacak."]);
    ar.said("Map 30 saniye sonra başlayacak.", at(96));
    out.clear();
    ar.tick(&w.snap(at(116)), &mut out);
    assert_eq!(sent(&out), ["Map 10 saniye sonra başlayacak."]);
    ar.said("Map 10 saniye sonra başlayacak.", at(116));
    out.clear();
    // The room was told 126; the bot waits ten seconds more.
    ar.tick(&w.snap(at(130)), &mut out);
    assert!(out.is_empty());
    ar.tick(&w.snap(at(136)), &mut out);
    assert_eq!(sent(&out)[0], "Süre doldu, map başlıyor.");
    assert!(sent(&out).contains(&"!mp start 10"));
}

#[test]
fn a_pick_marked_by_hand_is_played_on_resuming() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    // B's pick of HD1, marked on the referee page while the match was manual; A's turn next.
    w.picks.push(PickRow {
        map_id: Uuid::from_u128(20),
        team_id: B,
        created_at: at(-10),
    });
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(A),
    });
    let mut ar = AutoRef {
        phase: Phase::Draft {
            kind: StepKind::Pick,
            team_id: B,
            candidate: None,
        },
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.resume(&w.snap(at(0)), &mut out);
    ar.tick(&w.snap(at(1)), &mut out);
    assert!(matches!(ar.phase, Phase::Ready { map_id, .. } if map_id == Uuid::from_u128(20)));
    assert!(sent(&out)
        .iter()
        .any(|t| t.starts_with("Sıradaki map: HD1.")));
    // Once it's played, the next turn is asked.
    w.played.insert(Uuid::from_u128(20));
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    out.clear();
    ar.tick(&w.snap(at(2)), &mut out);
    assert!(matches!(ar.phase, Phase::Draft { team_id: A, .. }));
}

#[test]
fn a_pick_is_confirmed_checked_and_started() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(A),
    });
    w.picks.push(PickRow {
        map_id: Uuid::from_u128(20),
        team_id: A,
        created_at: at(-10),
    });
    w.played.insert(Uuid::from_u128(20));
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    assert!(matches!(ar.phase, Phase::Draft { team_id: A, .. }));
    // The other team can't answer; the same mod twice in a row is refused.
    out.clear();
    ar.player(&w.snap(at(1)), "b1", "fm1", true, &mut out);
    assert!(out.is_empty());
    ar.player(&w.snap(at(2)), "a1", "hd2", true, &mut out);
    assert!(sent(&out)[0].starts_with("Aynı moddan"));
    out.clear();
    ar.player(&w.snap(at(3)), "a1", "FM1", true, &mut out);
    ar.player(&w.snap(at(4)), "a2", "fm 1", true, &mut out);
    assert!(out.contains(&Action::Pick {
        map_id: Uuid::from_u128(30),
        team_id: A
    }));
    assert!(sent(&out).contains(&"!mp mods Freemod"));
    // Everyone ready: a freemod map checks NoFail first.
    out.clear();
    ar.bancho(&w.snap(at(10)), "All players are ready", &mut out);
    assert_eq!(sent(&out), ["!mp settings"]);
    w.room.slots.iter_mut().for_each(|s| s.mods = "NF".into());
    w.room.slots[0].mods = "HD".into();
    out.clear();
    ar.tick(&w.snap(at(14)), &mut out);
    assert!(sent(&out)[0].starts_with("a1: NoFail"));
    // Fixed, and ready again: it starts.
    w.room.slots[0].mods = "HDNF".into();
    out.clear();
    ar.bancho(&w.snap(at(20)), "All players are ready", &mut out);
    ar.tick(&w.snap(at(24)), &mut out);
    assert!(sent(&out).contains(&"!mp start 10"));
}

#[test]
fn abort_rules() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::Ready);
    let mut ar = AutoRef {
        phase: Phase::Playing {
            map_id: Uuid::from_u128(10),
            results: 0,
        },
        ..AutoRef::default()
    };
    w.room.playing = true;
    w.room.map_started_at = Some(at(0));
    let mut out = Vec::new();
    ar.player(&w.snap(at(10)), "a1", "-abort", true, &mut out);
    assert_eq!(
        sent(&out),
        ["!mp abort", "Abort kullanıldı. 1 hakkınız kaldı."]
    );
    out.clear();
    ar.player(&w.snap(at(40)), "a1", "-abort", true, &mut out);
    assert_eq!(
        sent(&out),
        ["Maç başladıktan 30 saniye sonra abort kullanılamaz."]
    );
    // A drop in the first 30 seconds uses the team's last one.
    out.clear();
    ar.bancho(&w.snap(at(5)), "a2 left the game.", &mut out);
    assert_eq!(ar.aborts_left(A), 0);
    out.clear();
    ar.player(&w.snap(at(6)), "a1", "-abort", true, &mut out);
    assert!(sent(&out)[0].contains("hakkı kalmadı"));
}

#[test]
fn stop_goes_to_manual_any_number_of_times() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.enter_ready(&w.snap(at(0)), &w.entries[0].clone(), &mut out);
    out.clear();
    ar.player(&w.snap(at(1)), "b1", "-durdur", true, &mut out);
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Stop { username, .. } if username == "b1")));
    // The phase stays, for a switch back to auto to pick up.
    assert!(matches!(ar.phase, Phase::Ready { .. }));
    // In manual mode too, and again by the same team; only flooding is held back.
    out.clear();
    ar.player(&w.snap(at(2)), "b2", "-durdur", false, &mut out);
    assert!(out.iter().any(|a| matches!(a, Action::Stop { .. })));
    out.clear();
    ar.player(&w.snap(at(3)), "b2", "-durdur", false, &mut out);
    assert!(out.is_empty());
    ar.player(&w.snap(at(70)), "b2", "-durdur", false, &mut out);
    assert!(out.iter().any(|a| matches!(a, Action::Stop { .. })));
}

#[test]
fn reports_work_in_manual_mode_too() {
    let w = World::bracket();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.player(
        &w.snap(at(0)),
        "a1",
        "-rapor rakip hile yapıyor",
        false,
        &mut out,
    );
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Report { text, .. } if text == "rakip hile yapıyor")));
    out.clear();
    ar.player(&w.snap(at(10)), "a1", "-rapor tekrar", false, &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Report { .. })));
    out.clear();
    ar.player(&w.snap(at(10)), "a1", "-abort", false, &mut out);
    assert!(out.is_empty());
}

#[test]
fn winning_concludes_and_tiebreaker_opens_tb() {
    let mut w = World::bracket();
    w.score = Some((2, 2));
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    assert!(sent(&out).contains(&"!mp map 90 0"));
    w.score = Some((3, 2));
    ar.phase = Phase::Next;
    out.clear();
    ar.tick(&w.snap(at(1)), &mut out);
    assert!(out.contains(&Action::Conclude));
    assert_eq!(ar.phase, Phase::Done);
}

#[test]
fn state_round_trips() {
    let ar = AutoRef {
        phase: Phase::Ready {
            map_id: A,
            until: at(5),
            nf_warned: true,
            check_at: None,
            forced: false,
            starting: Some(at(3)),
        },
        uses: vec![Uses {
            team_id: B,
            aborts: 1,
        }],
        ..AutoRef::default()
    };
    let back: AutoRef = serde_json::from_value(serde_json::to_value(&ar).unwrap()).unwrap();
    assert_eq!(back.phase, ar.phase);
    assert_eq!(back.uses, ar.uses);
}
