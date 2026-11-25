use super::*;
use crate::bancho::room::{RoomSlot, SlotStatus, SlotTeam, MATCH_STARTED};
use crate::error::PlayerError;

const A: Uuid = Uuid::from_u128(1);
const B: Uuid = Uuid::from_u128(2);

fn at(secs: i64) -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + Duration::hours(1000) + Duration::seconds(secs)
}

// The site's id of the player with this osu! id.
fn uid(osu_id: i64) -> Uuid {
    Uuid::from_u128((osu_id + 1000) as u128)
}

const PAGE: &str = "https://site/match/m/players";

fn player(name: &str, osu_id: i64, team: Uuid) -> RosterPlayer {
    RosterPlayer {
        user_id: uid(osu_id),
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
        beatmapset_id: id as i64 + 1000,
        mods: String::new(),
        mode: "osu".into(),
        freemod,
        tiebreaker: tb,
        label: label.into(),
        category: category.into(),
        repeatable: category == "NM",
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
    missed: Vec<MissedRow>,
    played: HashSet<Uuid>,
    room: RoomState,
    turn: Option<Turn>,
    score: Option<(i32, i32)>,
    bracket: bool,
    stand_ins: bool,
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
            missed: Vec::new(),
            played: HashSet::new(),
            // The bot locked it as it opened it.
            room: RoomState {
                locked: Some(true),
                ..RoomState::default()
            },
            turn: None,
            score: Some((0, 0)),
            bracket: true,
            stand_ins: false,
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
            missed: &self.missed,
            turn: self.turn,
            score: self.score,
            best_of: Some(5),
            played: &self.played,
            finished: &self.played,
            room: &self.room,
            no_fail: true,
            first_step_team: 1,
            page: PAGE,
            stand_ins: self.stand_ins,
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
fn starts_once_everyone_is_in_before_its_time() {
    // Being in the room is being ready: nobody has to press Ready.
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(-120)), &mut out);
    // Nobody gets the host: the team chooses its warmup on the room page.
    assert!(!sent(&out).iter().any(|t| t.starts_with("!mp host")));
    assert!(sent(&out)
        .iter()
        .any(|t| t.starts_with("Alfa takımının warmup sırası") && t.contains(PAGE)));
    assert!(matches!(ar.phase, Phase::Warmup { team: 0, .. }));
}

#[test]
fn warmups_then_roll_then_first_turn() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    // The second team skips ahead of its turn; the first chooses one and plays it.
    let skip = ar.web(&w.snap(at(5)), uid(4), Web::SkipWarmup, true, &mut out);
    assert_eq!(skip, Ok(()));
    assert_eq!(ar.view(&w.snap(at(5)), true).warmup_skipped, vec![B]);
    // Typed in the room, it does nothing now.
    out.clear();
    ar.player(&w.snap(at(6)), "a1", "!warmup-gec", true, &mut out);
    assert!(out.is_empty());
    let map = WarmupMap {
        seconds: 120,
        beatmapset_id: 9,
    };
    let chose = ar.web(
        &w.snap(at(8)),
        uid(1),
        Web::Warmup(555, map),
        true,
        &mut out,
    );
    assert_eq!(chose, Ok(()));
    assert!(sent(&out).contains(&"!mp map 555 0"));
    w.room.beatmap_id = Some(555);
    w.room.playing = true;
    w.room.map_started_at = Some(at(10));
    ar.bancho(&w.snap(at(10)), MATCH_STARTED, &mut out);
    w.room.playing = false;
    out.clear();
    ar.bancho(&w.snap(at(100)), "The match has finished!", &mut out);
    assert!(matches!(ar.phase, Phase::Roll { .. }));
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
            until: at(100),
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
    // The other team can't answer; the same mod twice in a row is refused; typed in the
    // room, a slot isn't taken.
    out.clear();
    let fm1 = Uuid::from_u128(30);
    let hd2 = Uuid::from_u128(21);
    assert_eq!(
        ar.web(&w.snap(at(1)), uid(3), Web::Draft(fm1), true, &mut out),
        Err(PlayerError::NotYourTurn)
    );
    assert_eq!(
        ar.web(&w.snap(at(2)), uid(1), Web::Draft(hd2), true, &mut out),
        Err(PlayerError::SameCategory)
    );
    ar.player(&w.snap(at(3)), "a1", "FM1", true, &mut out);
    ar.player(&w.snap(at(3)), "a1", "FM1", true, &mut out);
    assert!(out.is_empty());
    // In manual mode the page's pick isn't taken either.
    assert_eq!(
        ar.web(&w.snap(at(3)), uid(2), Web::Draft(fm1), false, &mut out),
        Err(PlayerError::Manual)
    );
    // Held on the page, it's taken at once.
    assert_eq!(
        ar.web(&w.snap(at(4)), uid(2), Web::Draft(fm1), true, &mut out),
        Ok(())
    );
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
    // Beta has nobody on HD, HR, EZ or FL: an FM map needs one per team.
    assert!(sent(&out)[1].starts_with("Beta: FM mapinde"));
    // Fixed, and ready again: it starts.
    w.room.slots[0].mods = "HDNF".into();
    w.room.slots[2].mods = "EZNF".into();
    out.clear();
    ar.bancho(&w.snap(at(20)), "All players are ready", &mut out);
    ar.tick(&w.snap(at(24)), &mut out);
    assert!(sent(&out).contains(&"!mp start 10"));
}

#[test]
fn a_ban_that_runs_out_is_lost() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Ban,
        team_id: Some(A),
    });
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    assert_eq!(
        sent(&out),
        [
            "Alfa takımı ban sırası, mapinizi maç odası sayfasından seçin: \
             https://site/match/m/players 3 dakika süreniz var, süre dolarsa ban hakkınız yanar."
        ]
    );
    assert_eq!(ar.view(&w.snap(at(0)), true).deadline, Some(at(190)));
    // A slot typed in the room doesn't stop the clock.
    out.clear();
    ar.player(&w.snap(at(100)), "a1", "hd1", true, &mut out);
    ar.player(&w.snap(at(101)), "a1", "hd1", true, &mut out);
    for t in 1..190 {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    assert!(sent(&out).contains(&"Alfa: ban için 1 dakika kaldı."));
    assert!(!out.iter().any(|a| matches!(a, Action::Missed { .. })));
    ar.tick(&w.snap(at(190)), &mut out);
    assert!(sent(&out).contains(&"Alfa takımının ban süresi doldu, ban hakkı yandı."));
    assert!(out.contains(&Action::Missed {
        team_id: A,
        kind: StepKind::Ban
    }));
    assert_eq!(ar.phase, Phase::Next);
}

#[test]
fn a_pick_that_runs_out_passes_then_goes_to_a_referee() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(A),
    });
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    assert!(sent(&out)[0].ends_with("2 dakika süreniz var, süre dolarsa pick sırası rakibe geçer."));
    out.clear();
    for t in 1..=130 {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    assert!(
        sent(&out).contains(&"Alfa takımının pick süresi doldu, pick sırası Beta takımına geçti.")
    );
    assert!(out.contains(&Action::Missed {
        team_id: A,
        kind: StepKind::Pick
    }));
    // The site records it, and the pick is Beta's: no second pass.
    w.missed.push(MissedRow {
        id: Uuid::from_u128(500),
        team_id: A,
        kind: StepKind::Pick,
        created_at: at(130),
    });
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(B),
    });
    out.clear();
    ar.tick(&w.snap(at(131)), &mut out);
    assert!(matches!(ar.phase, Phase::Draft { team_id: B, .. }));
    assert_eq!(
        sent(&out),
        [
            "Beta takımı pick sırası, mapinizi maç odası sayfasından seçin: \
             https://site/match/m/players 2 dakika süreniz var."
        ]
    );
    out.clear();
    for t in 132..=261 {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    assert!(!out.iter().any(|a| matches!(a, Action::Missed { .. })));
    assert!(out.iter().any(|a| matches!(a, Action::Handover(_))));
}

#[test]
fn a_turn_changed_by_a_referee_is_asked_again() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(B),
    });
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.tick(&w.snap(at(0)), &mut out);
    assert!(matches!(ar.phase, Phase::Draft { team_id: B, .. }));
    // A referee took back A's missed pick: it's A's turn again, on a new clock.
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(A),
    });
    out.clear();
    ar.tick(&w.snap(at(100)), &mut out);
    assert!(matches!(ar.phase, Phase::Draft { team_id: A, until, .. } if until == at(230)));
    assert!(sent(&out)[0].starts_with("Alfa takımı pick sırası"));
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
    ar.player(&w.snap(at(10)), "a1", "!abort", true, &mut out);
    assert_eq!(
        sent(&out),
        ["!mp abort", "a1 abort kullandı. 1 hakkınız kaldı."]
    );
    out.clear();
    ar.player(&w.snap(at(40)), "a1", "!ABORT", true, &mut out);
    assert_eq!(
        sent(&out),
        ["Maç başladıktan 30 saniye sonra abort kullanılamaz."]
    );
    // A drop in the first 30 seconds uses the team's last one.
    out.clear();
    ar.bancho(&w.snap(at(5)), "a2 left the game.", &mut out);
    assert_eq!(ar.aborts_left(A), 0);
    out.clear();
    ar.player(&w.snap(at(6)), "a1", "!abort", true, &mut out);
    assert!(sent(&out)[0].contains("hakkı kalmadı"));
    // From the page, the same refusal comes back to it instead.
    out.clear();
    assert_eq!(
        ar.web(&w.snap(at(6)), uid(1), Web::Abort, true, &mut out),
        Err(PlayerError::NoAbortsLeft)
    );
    assert!(out.is_empty());
    assert_eq!(
        ar.web(&w.snap(at(7)), uid(3), Web::Abort, true, &mut out),
        Ok(())
    );
    assert_eq!(ar.aborts_left(B), 1);
}

#[test]
fn stop_goes_to_manual_any_number_of_times() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.enter_ready(&w.snap(at(0)), &w.entries[0].clone(), &mut out);
    out.clear();
    ar.player(&w.snap(at(1)), "b1", "!panik", true, &mut out);
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Stop { username, .. } if username == "b1")));
    // The phase stays, for a switch back to auto to pick up.
    assert!(matches!(ar.phase, Phase::Ready { .. }));
    // In manual mode too, and again by the same team; only flooding is held back.
    out.clear();
    ar.player(&w.snap(at(2)), "b2", "!panic", false, &mut out);
    assert!(out.iter().any(|a| matches!(a, Action::Stop { .. })));
    out.clear();
    ar.player(&w.snap(at(3)), "b2", "!durdur", false, &mut out);
    assert!(out.is_empty());
    ar.player(&w.snap(at(70)), "b2", "-durdur", false, &mut out);
    assert!(out.iter().any(|a| matches!(a, Action::Stop { .. })));
    // From the page it's the same, and the page hears it's too soon.
    out.clear();
    assert_eq!(
        ar.web(&w.snap(at(71)), uid(4), Web::Stop, false, &mut out),
        Err(PlayerError::TooSoon)
    );
    assert_eq!(
        ar.web(&w.snap(at(71)), uid(1), Web::Stop, true, &mut out),
        Ok(())
    );
    // Someone who isn't playing in the match can't.
    assert_eq!(
        ar.web(&w.snap(at(72)), uid(99), Web::Stop, true, &mut out),
        Err(PlayerError::NotInMatch)
    );
}

#[test]
fn reports_work_in_manual_mode_too() {
    let w = World::bracket();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.player(
        &w.snap(at(0)),
        "a1",
        "!rapor rakip hile yapıyor",
        false,
        &mut out,
    );
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Report { text, .. } if text == "rakip hile yapıyor")));
    out.clear();
    ar.player(&w.snap(at(10)), "a1", "!report tekrar", false, &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Report { .. })));
    out.clear();
    ar.player(&w.snap(at(10)), "a1", "!abort", false, &mut out);
    assert!(out.is_empty());
    assert_eq!(
        ar.web(
            &w.snap(at(11)),
            uid(1),
            Web::Report("x".into()),
            false,
            &mut out
        ),
        Err(PlayerError::TooSoon)
    );
    assert_eq!(
        ar.web(
            &w.snap(at(11)),
            uid(3),
            Web::Report("  ".into()),
            false,
            &mut out
        ),
        Err(PlayerError::ReportEmpty)
    );
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
            fm_warned: true,
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

// A pick on an FM map: everyone ready with NoFail but Beta without a mod.
fn fm_ready(w: &mut World) -> AutoRef {
    w.everyone_in(SlotStatus::Ready);
    w.room.slots.iter_mut().for_each(|s| s.mods = "NF".into());
    w.room.slots[0].mods = "HRNF".into();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.enter_ready(&w.snap(at(0)), &w.entries[4].clone(), &mut out);
    ar
}

#[test]
fn a_team_without_an_fm_mod_stops_the_match_when_time_runs_out() {
    let mut w = World::bracket();
    let mut ar = fm_ready(&mut w);
    // The timer runs out: mods are read, Beta is told and gets the grace; still nothing
    // when it runs out, and the match stops for a referee instead of starting.
    let mut out = Vec::new();
    for t in 1..260 {
        ar.tick(&w.snap(at(t)), &mut out);
        if out.iter().any(|a| matches!(a, Action::Handover(_))) {
            break;
        }
    }
    assert_eq!(ar.phase, Phase::Next);
    let lines = sent(&out);
    // Each of these, in this order.
    let mut rest = lines.iter();
    for want in [
        "Süre doldu, map başlıyor.",
        "!mp settings",
        "Beta: FM mapinde",
        "Map 30 saniye",
        "!mp settings",
        "Beta: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. Seçilmediği",
    ] {
        assert!(rest.any(|l| l.starts_with(want)), "{want} in {lines:?}");
    }
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Handover(r) if r.contains("Beta"))));
    assert!(!lines.contains(&"!mp start 10"));
}

#[test]
fn the_tiebreaker_needs_no_mod() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::Ready);
    w.room.slots.iter_mut().for_each(|s| s.mods = "NF".into());
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.enter_ready(&w.snap(at(0)), &w.entries[5].clone(), &mut out);
    out.clear();
    ar.bancho(&w.snap(at(5)), "All players are ready", &mut out);
    ar.tick(&w.snap(at(9)), &mut out);
    assert!(sent(&out).contains(&"!mp start 10"));
}

#[test]
fn keeps_the_room_locked() {
    let mut w = World::bracket();
    w.room.locked = Some(false);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(-100)), &mut out);
    assert_eq!(sent(&out), ["!mp lock"]);
    // Not asked again straight away; again once a while passed and it's still open.
    out.clear();
    ar.tick(&w.snap(at(-90)), &mut out);
    assert!(sent(&out).is_empty());
    ar.tick(&w.snap(at(-60)), &mut out);
    assert_eq!(sent(&out), ["!mp lock"]);
    w.room.locked = Some(true);
    out.clear();
    ar.tick(&w.snap(at(-30)), &mut out);
    assert!(sent(&out).is_empty());
}

fn in_warmup(w: &mut World) -> AutoRef {
    w.everyone_in(SlotStatus::NotReady);
    AutoRef {
        phase: Phase::Warmup {
            team: 0,
            until: at(180),
            played: false,
            skipped: [false, false],
            beatmap: None,
            starting: false,
        },
        ..AutoRef::default()
    }
}

#[test]
fn a_warmup_starts_once_everyone_is_ready() {
    let mut w = World::bracket();
    let mut ar = in_warmup(&mut w);
    let mut out = Vec::new();
    // Ready before the team chose one: nothing starts.
    w.room
        .slots
        .iter_mut()
        .for_each(|s| s.status = SlotStatus::Ready);
    w.room.beatmap_id = Some(444);
    ar.bancho(&w.snap(at(2)), "All players are ready", &mut out);
    assert!(out.is_empty());
    // Only the team whose turn it is chooses.
    assert_eq!(
        ar.check_warmup(&w.snap(at(3)), uid(3)),
        Err(PlayerError::NotYourWarmup)
    );
    assert_eq!(ar.check_warmup(&w.snap(at(3)), uid(2)), Ok(()));
    let map = WarmupMap {
        seconds: 120,
        beatmapset_id: 9,
    };
    assert_eq!(
        ar.web(
            &w.snap(at(5)),
            uid(2),
            Web::Warmup(555, map),
            true,
            &mut out
        ),
        Ok(())
    );
    assert_eq!(
        sent(&out),
        [
            "!mp map 555 0",
            "!mp mods Freemod",
            "Alfa warmup mapini seçti. Herkes hazır olduğunda ya da süre dolunca başlayacak."
        ]
    );
    assert_eq!(ar.view(&w.snap(at(5)), true).warmup_beatmap_id, Some(555));
    w.room.beatmap_id = Some(555);
    out.clear();
    ar.bancho(&w.snap(at(10)), "All players are ready", &mut out);
    assert_eq!(
        sent(&out),
        ["Herkes hazır, warmup başlıyor.", "!mp start 10"]
    );
}

#[test]
fn a_warmup_chosen_starts_when_its_turn_runs_out() {
    let mut w = World::bracket();
    let mut ar = in_warmup(&mut w);
    let mut out = Vec::new();
    let map = WarmupMap {
        seconds: 200,
        beatmapset_id: 9,
    };
    // Chosen with 20 seconds left: everyone gets a minute.
    assert_eq!(
        ar.web(
            &w.snap(at(160)),
            uid(1),
            Web::Warmup(555, map),
            true,
            &mut out
        ),
        Ok(())
    );
    assert!(sent(&out)
        .iter()
        .any(|t| t.ends_with("ya da 1 dakika sonra başlayacak.")));
    w.room.beatmap_id = Some(555);
    out.clear();
    let mut started = None;
    for t in 161..300 {
        ar.tick(&w.snap(at(t)), &mut out);
        if started.is_none() && sent(&out).contains(&"!mp start 10") {
            started = Some(t);
        }
    }
    // Started once the minute (and the grace) ran out, not before.
    assert!(started.is_some_and(|t| t >= 230));
    assert!(sent(&out).contains(&"Warmup 30 saniye sonra başlayacak."));
    // It never started: the next team's turn came.
    assert!(matches!(ar.phase, Phase::Warmup { team: 1, .. }));
}

#[test]
fn a_pool_song_is_no_warmup_in_any_difficulty() {
    let mut w = World::bracket();
    let mut ar = in_warmup(&mut w);
    let mut out = Vec::new();
    // Another difficulty of HD1's set (1020), or too long: refused, nothing put on.
    let other_diff = WarmupMap {
        seconds: 90,
        beatmapset_id: 1020,
    };
    assert_eq!(
        ar.web(
            &w.snap(at(5)),
            uid(1),
            Web::Warmup(777, other_diff),
            true,
            &mut out
        ),
        Err(PlayerError::WarmupInPool)
    );
    let long = WarmupMap {
        seconds: 300,
        beatmapset_id: 5,
    };
    assert_eq!(
        ar.web(
            &w.snap(at(5)),
            uid(1),
            Web::Warmup(778, long),
            true,
            &mut out
        ),
        Err(PlayerError::WarmupTooLong)
    );
    assert!(out.is_empty());
    // A referee putting it on anyway: started, it's aborted.
    w.room.beatmap_id = Some(777);
    w.room.playing = true;
    ar.bancho(&w.snap(at(12)), MATCH_STARTED, &mut out);
    assert_eq!(sent(&out)[0], "!mp abort");
}

#[test]
fn resuming_follows_a_pool_map_being_played() {
    // A lobby a referee started by hand, then switched to auto mid-map.
    let mut w = World::bracket();
    w.bracket = false;
    w.everyone_in(SlotStatus::Ready);
    w.room.beatmap_id = Some(10);
    w.room.playing = true;
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.resume(&w.snap(at(30)), &mut out);
    assert!(matches!(ar.phase, Phase::Playing { map_id, .. } if map_id == Uuid::from_u128(10)));
    ar.tick(&w.snap(at(31)), &mut out);
    assert!(sent(&out).is_empty());
    // It finishes: the scores are read, then the next map comes.
    w.room.playing = false;
    ar.bancho(&w.snap(at(200)), "The match has finished!", &mut out);
    assert!(matches!(ar.phase, Phase::Scoring { .. }));
}

#[test]
fn resuming_a_match_the_referee_started_goes_to_its_next_step() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    // The referee did the warmups and the roll: A bans first.
    w.turn = Some(Turn {
        kind: StepKind::Ban,
        team_id: Some(A),
    });
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.resume(&w.snap(at(30)), &mut out);
    assert_eq!(ar.phase, Phase::Next);
    ar.tick(&w.snap(at(31)), &mut out);
    assert!(matches!(
        ar.phase,
        Phase::Draft {
            team_id: A,
            kind: StepKind::Ban,
            ..
        }
    ));
    assert!(!sent(&out).iter().any(|t| t.contains("warmup")));
}

#[test]
fn a_map_the_referee_put_on_for_a_pick_is_that_pick() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(B),
    });
    w.room.beatmap_id = Some(21);
    let mut ar = AutoRef {
        phase: Phase::Draft {
            kind: StepKind::Pick,
            team_id: B,
            until: at(10),
        },
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.resume(&w.snap(at(300)), &mut out);
    assert!(out.contains(&Action::Pick {
        map_id: Uuid::from_u128(21),
        team_id: B
    }));
    assert!(matches!(ar.phase, Phase::Ready { map_id, .. } if map_id == Uuid::from_u128(21)));
    // A map already played isn't taken for one.
    let mut w = World::bracket();
    w.turn = Some(Turn {
        kind: StepKind::Pick,
        team_id: Some(B),
    });
    w.room.beatmap_id = Some(21);
    w.played.insert(Uuid::from_u128(21));
    let mut ar = AutoRef {
        phase: Phase::Draft {
            kind: StepKind::Pick,
            team_id: B,
            until: at(10),
        },
        ..AutoRef::default()
    };
    out.clear();
    ar.resume(&w.snap(at(300)), &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Pick { .. })));
    assert_eq!(ar.phase, Phase::Next);
}

#[test]
fn a_pool_map_started_by_someone_else_is_followed() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::Ready);
    w.room.beatmap_id = Some(11);
    w.room.playing = true;
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    ar.bancho(&w.snap(at(0)), MATCH_STARTED, &mut out);
    assert!(matches!(ar.phase, Phase::Playing { map_id, .. } if map_id == Uuid::from_u128(11)));
}

// DEV_LOGIN: one real player (a1) with dummies (negative osu! ids) for the other three.
fn stand_in_world(stand_ins: bool) -> World {
    let mut w = World::bracket();
    w.roster = vec![
        player("a1", 1, A),
        player("DevA2", -2, A),
        player("DevB1", -3, B),
        player("DevB2", -4, B),
    ];
    w.room.slots = vec![slot(1, "a1", 1, SlotStatus::NotReady, "")];
    w.stand_ins = stand_ins;
    w
}

#[test]
fn stand_ins_count_as_in_the_room() {
    // Without DEV_LOGIN the dummies are missing: the match waits for them.
    let w = stand_in_world(false);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(1)), &mut out);
    assert_eq!(ar.phase, Phase::Waiting);
    assert!(sent(&out).contains(&"!mp invite #-3"));
    assert!(!ar.view(&w.snap(at(1)), true).stand_ins);

    // With it, the one real player is everyone: the match begins at once, before its time.
    let w = stand_in_world(true);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.tick(&w.snap(at(-60)), &mut out);
    assert!(matches!(ar.phase, Phase::Warmup { team: 0, .. }));
    assert!(!sent(&out).iter().any(|t| t.starts_with("!mp invite")));
    assert!(ar.view(&w.snap(at(1)), true).stand_ins);

    // A map starts once the real player is ready, though the dummies never join.
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.enter_ready(&w.snap(at(0)), &w.entries[0].clone(), &mut out);
    out.clear();
    ar.bancho(&w.snap(at(5)), "All players are ready", &mut out);
    assert!(sent(&out).contains(&"!mp start 10"));
}

#[test]
fn a_team_of_stand_ins_rolls_by_itself() {
    // Team B is only dummies: it rolls as the roll opens, and a1's roll settles it.
    let w = stand_in_world(true);
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.start_roll(&w.snap(at(0)), "!roll", &mut out);
    let Phase::Roll { rolls, .. } = ar.phase else {
        panic!("not rolling: {:?}", ar.phase);
    };
    assert!(rolls[0].is_none());
    let b = rolls[1].expect("team B rolled");
    assert!((1..=100).contains(&b));
    assert!(sent(&out)
        .iter()
        .any(|t| t.starts_with(&format!("DevB1 adına roll: {b} "))));
    // Team A has a real player, so its dummy doesn't roll for it.
    let a = if b == 100 { 1 } else { 100 };
    ar.bancho(&w.snap(at(5)), &format!("a1 rolls {a} point(s)"), &mut out);
    let winner = if a > b { A } else { B };
    assert!(out.contains(&Action::FirstTurn(winner)));
    assert_eq!(ar.phase, Phase::Next);

    // Two teams of dummies settle it at once, without a tie.
    let mut w = stand_in_world(true);
    w.roster[0] = player("DevA1", -1, A);
    w.room.slots.clear();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.start_roll(&w.snap(at(0)), "!roll", &mut out);
    assert_eq!(ar.phase, Phase::Next);
    assert!(out.iter().any(|a| matches!(a, Action::FirstTurn(_))));
}

#[test]
fn commands_take_either_prefix_and_both_languages() {
    assert_eq!(
        command("!rapor  oyun çöktü "),
        Some((Command::Report, "oyun çöktü"))
    );
    assert_eq!(command("!Report x"), Some((Command::Report, "x")));
    assert_eq!(command("-rapor x"), Some((Command::Report, "x")));
    for word in ["!panik", "!PANIC", "!durdur", "-durdur"] {
        assert_eq!(command(word), Some((Command::Stop, "")), "{word}");
    }
    assert_eq!(command("!abort"), Some((Command::Abort, "")));
    assert_eq!(command("!davet"), Some((Command::Invite, "")));
    assert_eq!(command(" !invite "), Some((Command::Invite, "")));
    assert_eq!(command("!roll"), None);
    assert_eq!(command("!mp abort"), None);
    assert_eq!(command("rapor x"), None);
    assert_eq!(command("!"), None);
}

#[test]
fn a_renamed_player_is_known_by_their_slot() {
    // a1 renamed on osu! since signing in: the room shows the new name with their id.
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.room.slots[0].username = "a1_new".into();
    let mut ar = AutoRef::default();
    let mut out = Vec::new();
    ar.player(&w.snap(at(0)), "a1_new", "!rapor test", false, &mut out);
    assert!(out
        .iter()
        .any(|a| matches!(a, Action::Report { user_id, .. } if *user_id == uid(1))));
}

#[test]
fn a_hold_begun_in_time_is_waited_for() {
    let mut w = World::bracket();
    w.everyone_in(SlotStatus::NotReady);
    w.turn = Some(Turn {
        kind: StepKind::Ban,
        team_id: Some(A),
    });
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    for t in 0..187 {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    let hd1 = Uuid::from_u128(20);
    // The other team's hold doesn't count, nor does a map the team may not ban.
    assert_eq!(
        ar.web(&w.snap(at(187)), uid(3), Web::Hold(hd1), true, &mut out),
        Err(PlayerError::NotYourTurn)
    );
    assert!(ar
        .web(
            &w.snap(at(187)),
            uid(1),
            Web::Hold(Uuid::from_u128(90)),
            true,
            &mut out
        )
        .is_err());
    // a1 begins holding HD1 three seconds before the bot would act.
    assert_eq!(
        ar.web(&w.snap(at(187)), uid(1), Web::Hold(hd1), true, &mut out),
        Ok(())
    );
    out.clear();
    ar.tick(&w.snap(at(190)), &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Missed { .. })));
    // The hold's ban lands past the time and is taken.
    assert_eq!(
        ar.web(&w.snap(at(190)), uid(1), Web::Draft(hd1), true, &mut out),
        Ok(())
    );
    assert!(out.contains(&Action::Ban {
        map_id: hd1,
        team_id: A
    }));

    // A hold let go isn't waited for long.
    let mut ar = AutoRef {
        phase: Phase::Next,
        ..AutoRef::default()
    };
    let mut out = Vec::new();
    for t in 0..187 {
        ar.tick(&w.snap(at(t)), &mut out);
    }
    assert_eq!(
        ar.web(&w.snap(at(187)), uid(1), Web::Hold(hd1), true, &mut out),
        Ok(())
    );
    ar.tick(&w.snap(at(192)), &mut out);
    assert!(!out.iter().any(|a| matches!(a, Action::Missed { .. })));
    ar.tick(&w.snap(at(193)), &mut out);
    assert!(out.contains(&Action::Missed {
        team_id: A,
        kind: StepKind::Ban
    }));
}
