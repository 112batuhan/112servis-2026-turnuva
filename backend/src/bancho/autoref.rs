//! The auto referee: the room's bot runs the match by itself, from the moment everyone
//! is in the room to the end, the way a referee would. A qualifier lobby plays its pool
//! map by map; a bracket match has its warmups, the roll, the bans and picks the players
//! type in the room, each map started once everyone is ready, the score read from osu!
//! after every map and the match concluded once a team has won. Players steer it with
//! `-abort`, `-durdur` (back to manual), `-rapor`, `-warmup-gec` (and `-davet` in a private message, which
//! the manager answers).
//!
//! It is a state machine with no I/O: the manager (manager.rs) feeds it the room's lines,
//! the players' commands and the clock along with a `Snapshot` of the match, and does the
//! `Action`s it returns (lines for the room, bans and picks, a forfeit, a conclusion,
//! notifications). Its state is saved after every change (`match_autoref`), so a restart
//! or a referee switching the match to manual and back continues where it left off
//! (`resume`).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::{
    commands::{map_command, mods_command, player_ref},
    context::MatchTeam,
    irc::same_name,
    room::{parse_left, split_mods, RoomState, SlotStatus, MATCH_STARTED},
    tally::{race_of, score_message, PoolEntry, Race, RosterPlayer},
};
use crate::{
    db::{
        autoref::LogLevel,
        bracket::{BanRow, PickRow},
    },
    draft::{StepKind, Turn},
};

// How many maps each team can abort, by -abort or by dropping out early.
pub const ABORTS: u8 = 2;
// A map can be aborted this long after it starts.
const ABORT_WINDOW: Duration = Duration::seconds(30);
// How long after the match's time the bot waits for everyone to join.
pub const NO_SHOW_WAIT: Duration = Duration::minutes(10);
// The time each team has to open a warmup, and the longest one it may play.
const WARMUP_TIME: Duration = Duration::minutes(3);
const WARMUP_MAX_SECS: i32 = 4 * 60;
// A warmup played past this long is aborted, whatever its length said.
const WARMUP_PLAY_MAX: Duration = Duration::seconds(4 * 60 + 30);
const ROLL_TIME: Duration = Duration::minutes(2);
// The ready timer of a map, in seconds: a qualifier's, a bracket pick's, and the grace a
// player missing NoFail gets once it ran out.
const QUALIFIER_READY: i64 = 180;
const BRACKET_READY: i64 = 120;
const NF_GRACE: i64 = 30;
// !mp settings gets this long to list everyone's mods.
const MODS_CHECK: Duration = Duration::seconds(3);
// !mp start's countdown, and how long the bot waits for the map to start before trying
// again.
const START_COUNTDOWN: u32 = 10;
const START_TIMEOUT: Duration = Duration::seconds(25);
// osu! is asked for a map's scores this long after it finished, and asked again as often
// up to SCORE_TRIES times while it doesn't have them yet.
pub const SCORE_DELAY: Duration = Duration::seconds(3);
const SCORE_TRIES: u8 = 5;
// A player can send one -rapor this often, of at most REPORT_MAX characters.
const REPORT_EVERY: Duration = Duration::seconds(60);
pub const REPORT_MAX: usize = 300;
// The room's timer once the match is over; the room is closed when it ends
// (manager.rs, CLOSE_AFTER_CONCLUDED).
const CLOSE_TIMER: u32 = 30;

// What the manager is to do, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // A line in the room: something to say, or an !mp command.
    Send(String),
    Log(LogLevel, String),
    Ban {
        map_id: Uuid,
        team_id: Uuid,
    },
    Pick {
        map_id: Uuid,
        team_id: Uuid,
    },
    // The roll's result: the team that bans or picks first.
    FirstTurn(Uuid),
    // Read osu!'s record of the rooms; answered with `osu_read`.
    FetchOsu,
    // The team in this slot (1 or 2) didn't show up.
    Forfeit(i16),
    // These teams didn't show up to the lobby.
    RemoveTeams(Vec<Uuid>),
    Conclude,
    Report {
        user_id: Uuid,
        team_id: Uuid,
        text: String,
    },
    // A player's -durdur: the match goes to manual and the staff hear it.
    Stop {
        user_id: Uuid,
        team_id: Uuid,
        username: String,
    },
    // The bot can't go on by itself: the match goes to manual and the staff hear why.
    Handover(String),
    // Look the beatmap's length up; answered with `warmup_length`.
    WarmupLength(i64),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    // Before the match: waiting for its time and its players.
    #[default]
    Waiting,
    // Bracket: team `team` (0 or 1) has the host to open a warmup.
    Warmup {
        team: usize,
        #[serde(with = "time::serde::rfc3339")]
        until: OffsetDateTime,
        // They started one.
        played: bool,
        // Teams that said -warmup-gec before their turn.
        skipped: [bool; 2],
    },
    // Bracket: both teams' first !roll.
    Roll {
        #[serde(with = "time::serde::rfc3339")]
        until: OffsetDateTime,
        rolls: [Option<i32>; 2],
    },
    // Bracket: a team is to name its ban or pick; `candidate` waits for them to say it
    // again.
    Draft {
        team_id: Uuid,
        kind: StepKind,
        candidate: Option<Uuid>,
    },
    // A map is on, waiting for everyone to be ready or its timer.
    Ready {
        map_id: Uuid,
        #[serde(with = "time::serde::rfc3339")]
        until: OffsetDateTime,
        // The players missing NoFail were told.
        nf_warned: bool,
        // Waiting for !mp settings to read everyone's mods; `forced` when the timer ran
        // out.
        #[serde(default, with = "time::serde::rfc3339::option")]
        check_at: Option<OffsetDateTime>,
        #[serde(default)]
        forced: bool,
        // !mp start went out.
        #[serde(default, with = "time::serde::rfc3339::option")]
        starting: Option<OffsetDateTime>,
    },
    // A map is being played; `results` is how many maps the room had finished before.
    Playing {
        map_id: Uuid,
        results: usize,
    },
    // A map finished: osu! is asked for its scores.
    Scoring {
        map_id: Uuid,
        #[serde(with = "time::serde::rfc3339")]
        fetch_at: OffsetDateTime,
        tries: u8,
    },
    // Between maps: what comes next is read from the match.
    Next,
    // The match is over (or forfeited).
    Done,
}

// How many of its -abort uses a team has spent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uses {
    pub team_id: Uuid,
    pub aborts: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoRef {
    #[serde(default)]
    pub phase: Phase,
    #[serde(default)]
    pub uses: Vec<Uses>,
    // The match's time came while players were missing, and the room was told.
    #[serde(default)]
    pub called: bool,
    // When each player last sent -rapor, and -durdur, by osu! id.
    #[serde(skip)]
    reports: HashMap<i64, OffsetDateTime>,
    #[serde(skip)]
    stops: HashMap<i64, OffsetDateTime>,
    // Warmup beatmaps' lengths in seconds, as osu! gave them.
    #[serde(skip)]
    lengths: HashMap<i64, i32>,
}

// The match as the auto referee needs it, read by the manager each time it's asked.
pub struct Snapshot<'a> {
    pub now: OffsetDateTime,
    pub bracket: bool,
    pub starts_at: Option<OffsetDateTime>,
    // When the latest room was put on the match: a room made late doesn't count the
    // wait from the match's time.
    pub room_added_at: OffsetDateTime,
    // A bracket match's two teams (team 1, team 2), or a lobby's in signup order.
    pub teams: &'a [MatchTeam],
    pub roster: &'a [RosterPlayer],
    pub entries: &'a [PoolEntry],
    pub bans: &'a [BanRow],
    pub picks: &'a [PickRow],
    // Bracket: the next ban or pick.
    pub turn: Option<Turn>,
    // Bracket: the map wins so far.
    pub score: Option<(i32, i32)>,
    pub best_of: Option<i32>,
    // Pool maps finished in any room of the match.
    pub played: &'a HashSet<Uuid>,
    // The latest room.
    pub room: &'a RoomState,
    pub no_fail: bool,
    // The team (1 or 2) of the stage's first ban or pick: the roll's winner is 1.
    pub first_step_team: u8,
}

impl Snapshot<'_> {
    fn present(&self, p: &RosterPlayer) -> bool {
        self.room.slots.iter().any(|s| match s.user_id {
            Some(id) => id == p.osu_id,
            None => same_name(&s.username, &p.username),
        })
    }

    fn player(&self, name: &str) -> Option<&RosterPlayer> {
        self.roster.iter().find(|p| same_name(&p.username, name))
    }

    fn members(&self, team_id: Uuid) -> impl Iterator<Item = &RosterPlayer> {
        self.roster.iter().filter(move |p| p.team_id == team_id)
    }

    fn complete(&self, team_id: Uuid) -> bool {
        let mut members = self.members(team_id).peekable();
        members.peek().is_some() && members.all(|p| self.present(p))
    }

    fn everyone_present(&self) -> bool {
        self.roster.iter().all(|p| self.present(p))
    }

    fn team_name(&self, id: Uuid) -> &str {
        self.teams
            .iter()
            .find(|t| t.id == id)
            .map_or("?", |t| t.name.as_str())
    }

    fn entry(&self, map_id: Uuid) -> Option<&PoolEntry> {
        self.entries.iter().find(|e| e.map_id == map_id)
    }

    // The pool slot a player typed: "hd1", "HD 1", "tb".
    fn entry_named(&self, text: &str) -> Option<&PoolEntry> {
        let typed: String = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();
        if typed.is_empty() || typed.len() > 8 {
            return None;
        }
        self.entries
            .iter()
            .find(|e| e.label.to_uppercase().replace(' ', "") == typed)
    }

    // The latest the players have to join.
    pub fn deadline(&self) -> Option<OffsetDateTime> {
        self.starts_at
            .map(|t| t.max(self.room_added_at) + NO_SHOW_WAIT)
    }
}

fn send(out: &mut Vec<Action>, text: impl Into<String>) {
    out.push(Action::Send(text.into()));
}

fn log(out: &mut Vec<Action>, level: LogLevel, text: impl Into<String>) {
    out.push(Action::Log(level, text.into()));
}

fn kind_word(kind: StepKind) -> &'static str {
    match kind {
        StepKind::Ban => "ban",
        StepKind::Pick => "pick",
    }
}

// "Name rolls 57 point(s)": BanchoBot's answer to !roll.
pub fn parse_roll(text: &str) -> Option<(&str, i32)> {
    let body = text.strip_suffix(" point(s)")?;
    let at = body.rfind(" rolls ")?;
    let name = &body[..at];
    let n = &body[at + " rolls ".len()..];
    if name.is_empty() || n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((name, n.parse().ok()?))
}

// A category players may pick from twice in a row.
fn repeatable(category: &str) -> bool {
    category.starts_with("NM")
}

// The command a player wrote, lowercased, and what follows it.
fn command(text: &str) -> Option<(String, &str)> {
    let text = text.trim();
    if !text.starts_with('-') {
        return None;
    }
    let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    Some((word.to_lowercase().replace('ç', "c"), rest.trim()))
}

impl AutoRef {
    fn uses_mut(&mut self, team_id: Uuid) -> &mut Uses {
        if let Some(i) = self.uses.iter().position(|u| u.team_id == team_id) {
            return &mut self.uses[i];
        }
        self.uses.push(Uses { team_id, aborts: 0 });
        self.uses.last_mut().unwrap()
    }

    pub fn aborts_left(&self, team_id: Uuid) -> u8 {
        ABORTS.saturating_sub(
            self.uses
                .iter()
                .find(|u| u.team_id == team_id)
                .map_or(0, |u| u.aborts),
        )
    }

    // ---------- the clock ----------

    pub fn tick(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let now = s.now;
        match self.phase.clone() {
            Phase::Waiting => self.waiting(s, out),
            Phase::Warmup {
                team,
                until,
                played,
                ..
            } => {
                let started = s.room.map_started_at;
                if s.room.playing && started.is_some_and(|t| now - t > WARMUP_PLAY_MAX) {
                    send(out, "!mp abort");
                    send(out, "Warmup 4 dakikayı geçti, sonlandırıldı.");
                    log(
                        out,
                        LogLevel::Warn,
                        "Warmup 4,5 dakikayı geçtiği için iptal edildi.",
                    );
                    self.next_warmup(s, team, out);
                } else if !played && !s.room.playing && now >= until {
                    let name = s.teams.get(team).map_or("?", |t| t.name.as_str());
                    send(out, format!("{name} takımının warmup süresi doldu."));
                    log(
                        out,
                        LogLevel::Info,
                        format!("{name} warmup açmadı, süre doldu."),
                    );
                    self.next_warmup(s, team, out);
                }
            }
            Phase::Roll { until, .. } if now >= until => self.resolve_roll(s, out),
            Phase::Ready {
                until,
                check_at,
                forced,
                starting,
                ..
            } => {
                if let Some(at) = starting {
                    // The countdown went out but the map never started (a line lost, or a
                    // referee's !mp aborttimer): ask again.
                    if !s.room.playing && now - at > START_TIMEOUT {
                        log(out, LogLevel::Warn, "Map başlamadı, tekrar deneniyor.");
                        self.set_ready(|r| r.starting = None);
                        self.try_start(s, true, out);
                    }
                } else if let Some(at) = check_at {
                    if now >= at {
                        self.check_mods(s, forced, out);
                    }
                } else if now >= until {
                    self.try_start(s, true, out);
                }
            }
            Phase::Scoring {
                map_id,
                fetch_at,
                tries,
            } if now >= fetch_at => {
                out.push(Action::FetchOsu);
                // Asked again if the answer never comes.
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: now + Duration::seconds(30),
                    tries,
                };
            }
            Phase::Next => self.next_step(s, out),
            _ => {}
        }
    }

    // Before the match: once everyone is in (at its time, or earlier if they're all
    // ready), it begins; at its time the missing are invited again; ten minutes after,
    // whoever isn't complete is out.
    fn waiting(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if s.roster.is_empty() {
            return;
        }
        let now = s.now;
        let due = s.starts_at.is_none_or(|t| now >= t);
        if s.everyone_present() {
            let ready = s.room.slots.iter().all(|slot| {
                slot.status == SlotStatus::Ready
                    || !s.roster.iter().any(|p| {
                        slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username)
                    })
            });
            if due || ready {
                self.begin(s, out);
            }
            return;
        }
        if s.starts_at.is_some() && due && !self.called {
            self.called = true;
            let missing: Vec<&RosterPlayer> = s.roster.iter().filter(|p| !s.present(p)).collect();
            for p in &missing {
                send(out, format!("!mp invite {}", player_ref(p.osu_id)));
            }
            send(
                out,
                "Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takım maçı kaybeder.",
            );
            let names: Vec<&str> = missing.iter().map(|p| p.username.as_str()).collect();
            log(
                out,
                LogLevel::Info,
                format!(
                    "Maç saati geldi, eksikler tekrar davet edildi: {}",
                    names.join(", ")
                ),
            );
        }
        if s.deadline().is_some_and(|d| now >= d) {
            self.no_show(s, out);
        }
    }

    fn no_show(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let complete: Vec<bool> = s.teams.iter().map(|t| s.complete(t.id)).collect();
        if !complete.iter().any(|&c| c) {
            out.push(Action::Handover(
                "10 dakika beklendi ama hiçbir takım tam kadro gelmedi.".into(),
            ));
            return;
        }
        if s.bracket {
            let [a, b] = [&s.teams[0], &s.teams[1]];
            let (slot, winner, loser) = if complete[0] { (2, a, b) } else { (1, b, a) };
            send(
                out,
                format!(
                    "{} takımı 10 dakika içinde tam kadro gelmedi. Maç hükmen {} takımına verildi.",
                    loser.name, winner.name
                ),
            );
            log(
                out,
                LogLevel::Warn,
                format!("{} gelmedi: hükmen {} kazandı.", loser.name, winner.name),
            );
            out.push(Action::Forfeit(slot));
            self.phase = Phase::Done;
            return;
        }
        let gone: Vec<&MatchTeam> = s
            .teams
            .iter()
            .zip(&complete)
            .filter(|(_, c)| !**c)
            .map(|(t, _)| t)
            .collect();
        for team in &gone {
            for p in s.members(team.id).filter(|p| s.present(p)) {
                send(out, format!("!mp kick {}", player_ref(p.osu_id)));
            }
        }
        let names: Vec<&str> = gone.iter().map(|t| t.name.as_str()).collect();
        send(
            out,
            format!(
                "{} tam kadro gelmediği için lobiden çıkarıldı.",
                names.join(", ")
            ),
        );
        log(
            out,
            LogLevel::Warn,
            format!(
                "Tam kadro gelmeyen takımlar lobiden çıkarıldı: {}",
                names.join(", ")
            ),
        );
        out.push(Action::RemoveTeams(gone.iter().map(|t| t.id).collect()));
        self.begin(s, out);
    }

    fn begin(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        log(out, LogLevel::Info, "Herkes odada, maç başlıyor.");
        if s.bracket {
            self.enter_warmup(s, 0, [false, false], out);
        } else {
            send(
                out,
                "Herkes burada, maç başlıyor! Mapler sırayla açılacak, iyi şanslar.",
            );
            self.next_step(s, out);
        }
    }

    // ---------- warmups and the roll ----------

    fn enter_warmup(
        &mut self,
        s: &Snapshot,
        team: usize,
        skipped: [bool; 2],
        out: &mut Vec<Action>,
    ) {
        if team >= 2 || team >= s.teams.len() {
            send(out, "!mp clearhost");
            return self.enter_roll(s, out);
        }
        let t = &s.teams[team];
        if skipped[team] {
            log(out, LogLevel::Info, format!("{} warmupı geçti.", t.name));
            return self.enter_warmup(s, team + 1, skipped, out);
        }
        // The team's first player, or whichever of them is in the room.
        let host = s
            .members(t.id)
            .find(|p| s.present(p))
            .or_else(|| s.members(t.id).next());
        let Some(host) = host else {
            return self.enter_warmup(s, team + 1, skipped, out);
        };
        send(out, format!("!mp host {}", player_ref(host.osu_id)));
        send(
            out,
            format!(
                "{} takımına warmup açmaları için host yetkisi verildi. Warmup mapleri 4 dakikayı \
                 geçmemelidir. Eğer warmup oynamak istemiyorlarsa -warmup-gec yazabilirler.",
                t.name
            ),
        );
        send(out, "!mp timer 180");
        log(
            out,
            LogLevel::Info,
            format!("Warmup: host {} ({}).", host.username, t.name),
        );
        self.phase = Phase::Warmup {
            team,
            until: s.now + WARMUP_TIME,
            played: false,
            skipped,
        };
    }

    fn next_warmup(&mut self, s: &Snapshot, team: usize, out: &mut Vec<Action>) {
        let skipped = match &self.phase {
            Phase::Warmup { skipped, .. } => *skipped,
            _ => [false, false],
        };
        send(out, "!mp aborttimer");
        self.enter_warmup(s, team + 1, skipped, out);
    }

    fn enter_roll(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        send(
            out,
            "Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var.",
        );
        send(out, "!mp timer 120");
        log(out, LogLevel::Info, "Roll bekleniyor.");
        self.phase = Phase::Roll {
            until: s.now + ROLL_TIME,
            rolls: [None, None],
        };
    }

    fn take_roll(&mut self, s: &Snapshot, name: &str, n: i32, out: &mut Vec<Action>) {
        let Phase::Roll { until, mut rolls } = self.phase.clone() else {
            return;
        };
        let Some(p) = s.player(name) else {
            return;
        };
        let Some(i) = s.teams.iter().position(|t| t.id == p.team_id) else {
            return;
        };
        if rolls[i].is_some() {
            return;
        }
        if n > 100 {
            send(
                out,
                format!(
                    "{}: sadece !roll yazın (1-100), bu roll sayılmadı.",
                    p.username
                ),
            );
            return;
        }
        rolls[i] = Some(n);
        log(
            out,
            LogLevel::Info,
            format!("{} ({}) {n} attı.", p.username, s.teams[i].name),
        );
        self.phase = Phase::Roll { until, rolls };
        if rolls.iter().all(Option::is_some) {
            self.resolve_roll(s, out);
        }
    }

    fn resolve_roll(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Phase::Roll { rolls, .. } = self.phase.clone() else {
            return;
        };
        if s.teams.len() < 2 {
            return;
        }
        let (winner, line) = match rolls {
            [Some(a), Some(b)] if a == b => {
                send(
                    out,
                    format!("Berabere ({a} - {b})! İki takım da tekrar !roll yazsın."),
                );
                send(out, "!mp timer 120");
                self.phase = Phase::Roll {
                    until: s.now + ROLL_TIME,
                    rolls: [None, None],
                };
                return;
            }
            [Some(a), Some(b)] => (if a > b { 0 } else { 1 }, format!("({a} - {b})")),
            [Some(_), None] => (0, "(rakibi roll atmadı)".into()),
            [None, Some(_)] => (1, "(rakibi roll atmadı)".into()),
            [None, None] => (
                rand::random::<bool>() as usize,
                "(iki takım da roll atmadı, bot rastgele seçti)".into(),
            ),
        };
        let won = &s.teams[winner];
        let lost = &s.teams[1 - winner];
        send(out, "!mp aborttimer");
        send(out, format!("{} rollü kazandı {line}.", won.name));
        log(
            out,
            LogLevel::Info,
            format!("Roll: {} kazandı {line}.", won.name),
        );
        let first = if s.first_step_team == 1 { won } else { lost };
        out.push(Action::FirstTurn(first.id));
        self.phase = Phase::Next;
    }

    // ---------- between maps ----------

    fn next_step(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if !s.bracket {
            let next = s
                .entries
                .iter()
                .find(|e| !e.tiebreaker && !s.played.contains(&e.map_id));
            match next {
                Some(entry) => self.enter_ready(s, entry, out),
                None => self.finish(s, None, out),
            }
            return;
        }
        if s.teams.len() < 2 {
            out.push(Action::Handover("Maçın iki takımı belli değil.".into()));
            return;
        }
        if let (Some(best_of), Some((a, b))) = (s.best_of, s.score) {
            match race_of(best_of, a, b) {
                Race::Won => {
                    let winner = if a > b { &s.teams[0] } else { &s.teams[1] };
                    return self.finish(s, Some(winner), out);
                }
                Race::Over => {
                    out.push(Action::Handover(format!(
                        "Skor ({a} - {b}) Bo{best_of}'u aşıyor."
                    )));
                    return;
                }
                Race::Tiebreaker => {
                    let Some(tb) = s.entries.iter().find(|e| e.tiebreaker) else {
                        out.push(Action::Handover(
                            "Skor tiebreaker'a geldi ama havuzda tiebreaker yok.".into(),
                        ));
                        return;
                    };
                    if s.played.contains(&tb.map_id) {
                        out.push(Action::Handover(
                            "Tiebreaker oynandı ama kazanan belli değil.".into(),
                        ));
                        return;
                    }
                    send(out, "Tiebreaker!");
                    log(out, LogLevel::Info, "Skor eşit, tiebreaker açılıyor.");
                    return self.enter_ready(s, tb, out);
                }
                Race::Open | Race::MatchPoint => {}
            }
        }
        match s.turn {
            Some(Turn {
                kind,
                team_id: Some(team_id),
            }) => self.ask(s, team_id, kind, out),
            Some(_) => out.push(Action::Handover(
                "Sıradaki ban/pick'in takımı belirlenemedi.".into(),
            )),
            None if s.best_of.is_none() => out.push(Action::Handover(
                "Stage'in best of değeri yok, maçın ne zaman bittiği bilinmiyor.".into(),
            )),
            None => out.push(Action::Handover(
                "Stage'in pick sırası bitti ama maç bitmedi.".into(),
            )),
        }
    }

    fn ask(&mut self, s: &Snapshot, team_id: Uuid, kind: StepKind, out: &mut Vec<Action>) {
        let name = s.team_name(team_id);
        send(
            out,
            format!(
                "{name} takımı {} sırası, seçtiğiniz mapin slotunu söyleyin.",
                kind_word(kind)
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!("{name} takımının {} seçimi bekleniyor.", kind_word(kind)),
        );
        self.phase = Phase::Draft {
            team_id,
            kind,
            candidate: None,
        };
    }

    fn finish(&mut self, s: &Snapshot, winner: Option<&MatchTeam>, out: &mut Vec<Action>) {
        match (winner, s.score) {
            (Some(w), Some((a, b))) => {
                let names = [s.teams[0].name.as_str(), s.teams[1].name.as_str()];
                send(
                    out,
                    format!(
                        "{} maçı kazandı! {}",
                        w.name,
                        score_message(names, [a, b], None)
                    ),
                );
                log(
                    out,
                    LogLevel::Info,
                    format!("{} kazandı ({a} - {b}).", w.name),
                );
            }
            _ => {
                send(out, "Bütün mapler oynandı, katıldığınız için teşekkürler!");
                log(out, LogLevel::Info, "Bütün mapler oynandı.");
            }
        }
        send(out, "Maç bitti, oda 30 saniye sonra kapanacak.");
        send(out, format!("!mp timer {CLOSE_TIMER}"));
        out.push(Action::Conclude);
        self.phase = Phase::Done;
    }

    // ---------- a map ----------

    fn enter_ready(&mut self, s: &Snapshot, entry: &PoolEntry, out: &mut Vec<Action>) {
        let secs = if s.bracket {
            BRACKET_READY
        } else {
            QUALIFIER_READY
        };
        send(out, map_command(entry.beatmap_id, &entry.mode));
        send(out, mods_command(&entry.mods, entry.freemod, s.no_fail));
        send(out, format!("!mp timer {secs}"));
        send(
            out,
            format!(
                "Sıradaki map: {}. Herkes hazır olduğunda ya da sayaç sıfırlandığında map başlayacak.",
                entry.label
            ),
        );
        log(out, LogLevel::Info, format!("{} açıldı.", entry.label));
        self.phase = Phase::Ready {
            map_id: entry.map_id,
            until: s.now + Duration::seconds(secs),
            nf_warned: false,
            check_at: None,
            forced: false,
            starting: None,
        };
    }

    fn set_ready(&mut self, f: impl FnOnce(&mut ReadyFields)) {
        if let Phase::Ready {
            until,
            nf_warned,
            check_at,
            forced,
            starting,
            ..
        } = &mut self.phase
        {
            let mut fields = ReadyFields {
                until: *until,
                nf_warned: *nf_warned,
                check_at: *check_at,
                forced: *forced,
                starting: *starting,
            };
            f(&mut fields);
            *until = fields.until;
            *nf_warned = fields.nf_warned;
            *check_at = fields.check_at;
            *forced = fields.forced;
            *starting = fields.starting;
        }
    }

    // Everyone is ready (or the timer ran out): FM and TB maps check everyone's NoFail
    // first.
    fn try_start(&mut self, s: &Snapshot, forced: bool, out: &mut Vec<Action>) {
        let Phase::Ready {
            map_id,
            check_at,
            starting,
            ..
        } = self.phase.clone()
        else {
            return;
        };
        if check_at.is_some() || starting.is_some() {
            return;
        }
        // A bracket map waits for all four players unless its timer ran out.
        if !forced && s.bracket && !s.everyone_present() {
            return;
        }
        let freemod = s.entry(map_id).is_some_and(|e| e.freemod);
        if freemod && s.no_fail {
            send(out, "!mp settings");
            self.set_ready(|r| {
                r.check_at = Some(s.now + MODS_CHECK);
                r.forced = forced;
            });
            return;
        }
        self.start(s, out);
    }

    fn check_mods(&mut self, s: &Snapshot, forced: bool, out: &mut Vec<Action>) {
        let Phase::Ready { nf_warned, .. } = self.phase.clone() else {
            return;
        };
        self.set_ready(|r| r.check_at = None);
        let missing: Vec<&str> = s
            .room
            .slots
            .iter()
            .filter(|slot| {
                slot.mods_known
                    && !split_mods(&slot.mods).iter().any(|m| m == "NF")
                    && s.roster.iter().any(|p| {
                        slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username)
                    })
            })
            .map(|slot| slot.username.as_str())
            .collect();
        if missing.is_empty() {
            return self.start(s, out);
        }
        if forced && nf_warned {
            send(out, "Süre doldu, map NoFail'siz oyuncularla başlıyor.");
            log(
                out,
                LogLevel::Warn,
                format!("NF açmadan başladı: {}", missing.join(", ")),
            );
            return self.start(s, out);
        }
        send(
            out,
            format!(
                "{}: NoFail (NF) modunu seçmediniz, lütfen NF açın.",
                missing.join(", ")
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!("NF uyarısı: {}", missing.join(", ")),
        );
        self.set_ready(|r| r.nf_warned = true);
        if forced {
            send(out, format!("!mp timer {NF_GRACE}"));
            self.set_ready(|r| r.until = s.now + Duration::seconds(NF_GRACE));
        }
    }

    fn start(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        send(out, "!mp aborttimer");
        send(out, format!("!mp start {START_COUNTDOWN}"));
        self.set_ready(|r| r.starting = Some(s.now));
    }

    // ---------- what BanchoBot says ----------

    // A line BanchoBot said in the room, once the room's state has it.
    pub fn bancho(&mut self, s: &Snapshot, text: &str, out: &mut Vec<Action>) {
        if let Some((name, n)) = parse_roll(text) {
            return self.take_roll(s, name, n, out);
        }
        match (self.phase.clone(), text) {
            (Phase::Ready { .. }, "All players are ready") => self.try_start(s, false, out),
            (Phase::Ready { map_id, .. }, MATCH_STARTED) => {
                self.phase = Phase::Playing {
                    map_id,
                    results: s.room.results.len(),
                };
            }
            (Phase::Playing { .. }, "Aborted the match") => self.reopen(s, out),
            (Phase::Playing { map_id, .. }, "The match has finished!") => {
                log(
                    out,
                    LogLevel::Info,
                    "Map bitti, osu!'dan skorlar bekleniyor.",
                );
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now + SCORE_DELAY,
                    tries: 0,
                };
            }
            (Phase::Playing { .. }, _) => {
                if let Some(name) = parse_left(text) {
                    self.dropped(s, name, out);
                }
            }
            (Phase::Warmup { .. }, MATCH_STARTED) => self.warmup_started(s, out),
            (Phase::Warmup { team, .. }, "The match has finished!") => {
                self.next_warmup(s, team, out)
            }
            (Phase::Warmup { .. }, _)
                if text.starts_with("Changed beatmap to ")
                    || text.starts_with("Beatmap changed to: ") =>
            {
                self.warmup_map(s, out)
            }
            _ => {}
        }
    }

    // The map that was aborted goes back on.
    fn reopen(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Phase::Playing { map_id, .. } = self.phase.clone() else {
            return;
        };
        match s.entry(map_id) {
            Some(entry) => self.enter_ready(s, entry, out),
            None => self.phase = Phase::Next,
        }
    }

    // Someone left while a map was played: within its first 30 seconds, it's aborted on
    // their team's account (in a qualifier only on its first map; after that it plays on
    // without them).
    fn dropped(&mut self, s: &Snapshot, name: &str, out: &mut Vec<Action>) {
        let Some(p) = s.player(name) else {
            return;
        };
        let early = s
            .room
            .map_started_at
            .is_some_and(|t| s.now - t <= ABORT_WINDOW);
        if !early {
            return;
        }
        if !s.bracket && !s.played.is_empty() {
            log(
                out,
                LogLevel::Info,
                format!("{} düştü, maç onsuz devam ediyor.", p.username),
            );
            return;
        }
        let team_id = p.team_id;
        if self.aborts_left(team_id) == 0 {
            log(
                out,
                LogLevel::Info,
                format!("{} düştü ama takımının abort hakkı kalmadı.", p.username),
            );
            return;
        }
        self.uses_mut(team_id).aborts += 1;
        let left = self.aborts_left(team_id);
        send(out, "!mp abort");
        send(
            out,
            format!(
                "{} oyundan düştü. Abort kullanıldı. {left} hakkınız kaldı",
                p.username
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!(
                "{} düştüğü için map iptal edildi ({} hakkı kaldı).",
                p.username, left
            ),
        );
    }

    fn warmup_started(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let beatmap = s.room.beatmap_id;
        let in_pool = beatmap.is_some_and(|b| s.entries.iter().any(|e| e.beatmap_id == b));
        let too_long = beatmap
            .and_then(|b| self.lengths.get(&b))
            .is_some_and(|&secs| secs > WARMUP_MAX_SECS);
        if in_pool || too_long {
            send(out, "!mp abort");
            send(
                out,
                if in_pool {
                    "Havuzdaki mapler warmup olarak oynanamaz, başka bir map seçin."
                } else {
                    "Bu map 4 dakikadan uzun, başka bir warmup seçin."
                },
            );
            log(
                out,
                LogLevel::Warn,
                "Kurallara uymayan warmup iptal edildi.",
            );
            return;
        }
        if let Phase::Warmup { played, .. } = &mut self.phase {
            *played = true;
        }
    }

    fn warmup_map(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(beatmap) = s.room.beatmap_id else {
            return;
        };
        if s.entries.iter().any(|e| e.beatmap_id == beatmap) {
            send(
                out,
                "Havuzdaki mapler warmup olarak oynanamaz, başka bir map seçin.",
            );
            return;
        }
        match self.lengths.get(&beatmap) {
            Some(&secs) => self.warmup_too_long(s, secs, out),
            None => out.push(Action::WarmupLength(beatmap)),
        }
    }

    // osu! said how long a warmup beatmap is.
    pub fn warmup_length(&mut self, s: &Snapshot, beatmap: i64, secs: i32, out: &mut Vec<Action>) {
        self.lengths.insert(beatmap, secs);
        if matches!(self.phase, Phase::Warmup { .. }) && s.room.beatmap_id == Some(beatmap) {
            self.warmup_too_long(s, secs, out);
        }
    }

    fn warmup_too_long(&mut self, s: &Snapshot, secs: i32, out: &mut Vec<Action>) {
        if secs <= WARMUP_MAX_SECS {
            return;
        }
        send(
            out,
            format!(
                "Bu map {}:{:02} uzunluğunda, warmup 4 dakikayı geçmemeli. Başka bir map seçin.",
                secs / 60,
                secs % 60
            ),
        );
        if s.room.playing {
            send(out, "!mp abort");
        }
    }

    // osu!'s record of the rooms was read: `confirmed` when it has the map that just
    // finished.
    pub fn osu_read(&mut self, s: &Snapshot, confirmed: bool, out: &mut Vec<Action>) {
        let Phase::Scoring { map_id, tries, .. } = self.phase.clone() else {
            return;
        };
        if !confirmed {
            if tries + 1 < SCORE_TRIES {
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now + SCORE_DELAY,
                    tries: tries + 1,
                };
                return;
            }
            log(
                out,
                LogLevel::Warn,
                "osu! skorları getiremedi, sohbetteki skorlarla devam ediliyor.",
            );
        } else {
            log(out, LogLevel::Info, "Skorlar osu!'dan doğrulandı.");
        }
        // The map that decided the match is announced with the winner instead.
        let decided = matches!(
            (s.best_of, s.score),
            (Some(bo), Some((a, b))) if race_of(bo, a, b) == Race::Won
        );
        if let (true, false, Some((a, b))) = (s.bracket, decided, s.score) {
            if s.teams.len() == 2 {
                let names = [s.teams[0].name.as_str(), s.teams[1].name.as_str()];
                send(out, score_message(names, [a, b], None));
            }
        }
        self.phase = Phase::Next;
    }

    // ---------- what the players write ----------

    // A line a player wrote in the room. `enabled`: the match is in auto mode; -rapor
    // and -durdur work either way.
    pub fn player(
        &mut self,
        s: &Snapshot,
        name: &str,
        text: &str,
        enabled: bool,
        out: &mut Vec<Action>,
    ) {
        let Some(p) = s.player(name).cloned() else {
            return;
        };
        let command = command(text);
        if let Some(("-rapor", rest)) = command.as_ref().map(|(w, r)| (w.as_str(), *r)) {
            return self.report(s, &p, rest, out);
        }
        if command.as_ref().is_some_and(|(w, _)| w == "-durdur") {
            return self.stop(s, &p, out);
        }
        if !enabled || self.phase == Phase::Done {
            return;
        }
        match command.as_ref().map(|(w, _)| w.as_str()) {
            Some("-abort") => self.abort(s, &p, out),
            Some("-warmup-gec") => self.skip_warmup(s, &p, out),
            _ => self.draft_input(s, &p, text, out),
        }
    }

    fn report(&mut self, s: &Snapshot, p: &RosterPlayer, text: &str, out: &mut Vec<Action>) {
        if text.is_empty() {
            send(out, format!("{}: kullanım: -rapor <mesajınız>", p.username));
            return;
        }
        if self
            .reports
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            send(
                out,
                format!("{}: bir dakikada bir rapor gönderebilirsiniz.", p.username),
            );
            return;
        }
        self.reports.insert(p.osu_id, s.now);
        let text: String = text.chars().take(REPORT_MAX).collect();
        send(
            out,
            format!("{}: raporun hostlara ve hakemlere iletildi.", p.username),
        );
        log(
            out,
            LogLevel::Warn,
            format!("Rapor ({}): {text}", p.username),
        );
        out.push(Action::Report {
            user_id: p.user_id,
            team_id: p.team_id,
            text,
        });
    }

    // -durdur, for emergencies, any number of times: the bot stops (the match goes to
    // manual until a referee switches it back) and the staff hear it. Once a minute per
    // player, so it can't flood them.
    fn stop(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        if self
            .stops
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            return;
        }
        self.stops.insert(p.osu_id, s.now);
        // A countdown or a mods check waiting is dropped; it's set again on resuming.
        self.set_ready(|r| {
            r.check_at = None;
            r.starting = None;
        });
        send(out, "!mp aborttimer");
        send(
            out,
            "Durdur komutu kullanıldı. Bu komut sadece acil durumlarda kullanılmalıdır. Hostlar \
             bilgilendirildi, eğer bu komut kötü niyetle kullanıldıysa -rapor komutuyla bildiriniz",
        );
        out.push(Action::Stop {
            user_id: p.user_id,
            team_id: p.team_id,
            username: p.username.clone(),
        });
    }

    fn abort(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        if !matches!(self.phase, Phase::Playing { .. }) || !s.room.playing {
            send(out, format!("{}: şu an oynanan bir map yok.", p.username));
            return;
        }
        let early = s
            .room
            .map_started_at
            .is_some_and(|t| s.now - t <= ABORT_WINDOW);
        if !early {
            send(out, "Maç başladıktan 30 saniye sonra abort kullanılamaz");
            return;
        }
        if self.aborts_left(p.team_id) == 0 {
            send(
                out,
                format!("{}: takımınızın abort hakkı kalmadı.", p.username),
            );
            return;
        }
        self.uses_mut(p.team_id).aborts += 1;
        let left = self.aborts_left(p.team_id);
        send(out, "!mp abort");
        send(out, format!("Abort kullanıldı. {left} hakkınız kaldı"));
        log(
            out,
            LogLevel::Info,
            format!("{} -abort kullandı ({left} hakkı kaldı).", p.username),
        );
    }

    fn skip_warmup(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        let Phase::Warmup {
            team,
            until,
            played,
            mut skipped,
        } = self.phase.clone()
        else {
            return;
        };
        let Some(i) = s.teams.iter().position(|t| t.id == p.team_id) else {
            return;
        };
        let name = &s.teams[i].name;
        if i == team {
            if s.room.playing {
                return;
            }
            send(out, format!("{name} warmupı geçti."));
            log(out, LogLevel::Info, format!("{name} -warmup-gec yazdı."));
            self.next_warmup(s, team, out);
        } else if i > team && !skipped[i] {
            skipped[i] = true;
            send(out, format!("{name} sırası gelince warmupı geçecek."));
            self.phase = Phase::Warmup {
                team,
                until,
                played,
                skipped,
            };
        }
    }

    // A player of the team whose turn it is naming a slot ("HD1"), then naming it again
    // to confirm.
    fn draft_input(&mut self, s: &Snapshot, p: &RosterPlayer, text: &str, out: &mut Vec<Action>) {
        let Phase::Draft {
            team_id,
            kind,
            candidate,
        } = self.phase.clone()
        else {
            return;
        };
        if p.team_id != team_id {
            return;
        }
        let Some(entry) = s.entry_named(text) else {
            return;
        };
        if let Some(reason) = refusal(s, entry, kind, team_id) {
            send(out, reason);
            return;
        }
        if candidate != Some(entry.map_id) {
            let word = match kind {
                StepKind::Ban => "Ban",
                StepKind::Pick => "Pick",
            };
            send(
                out,
                format!(
                    "{word} için {} seçtiğinizi doğrulamak için tekrar yazın.",
                    entry.label
                ),
            );
            self.phase = Phase::Draft {
                team_id,
                kind,
                candidate: Some(entry.map_id),
            };
            return;
        }
        let name = s.team_name(team_id);
        let did = match kind {
            StepKind::Ban => "banladı",
            StepKind::Pick => "seçti",
        };
        log(
            out,
            LogLevel::Info,
            format!("{name} {} {did} ({} onayladı).", entry.label, p.username),
        );
        match kind {
            StepKind::Ban => {
                send(
                    out,
                    format!("{name} takımı {} mapini banladı.", entry.label),
                );
                out.push(Action::Ban {
                    map_id: entry.map_id,
                    team_id,
                });
                self.phase = Phase::Next;
            }
            StepKind::Pick => {
                send(out, format!("{name} takımı {} mapini seçti.", entry.label));
                out.push(Action::Pick {
                    map_id: entry.map_id,
                    team_id,
                });
                self.enter_ready(s, entry, out);
            }
        }
    }

    // ---------- coming back ----------

    // Picks up again after the match was in manual mode (a referee's switch or a
    // -durdur) or a restart: the phase's timers start over, and whatever happened in
    // between is read from the room.
    pub fn resume(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        log(
            out,
            LogLevel::Info,
            format!("Kaldığı yerden devam ediyor: {}.", phase_name(&self.phase)),
        );
        match self.phase.clone() {
            Phase::Waiting | Phase::Next | Phase::Done => {}
            Phase::Warmup { team, skipped, .. } => {
                if !s.room.playing {
                    self.enter_warmup(s, team, skipped, out);
                }
            }
            Phase::Roll { rolls, .. } => {
                send(
                    out,
                    "Pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var.",
                );
                send(out, "!mp timer 120");
                self.phase = Phase::Roll {
                    until: s.now + ROLL_TIME,
                    rolls,
                };
            }
            // The turn is read again: a referee may have marked bans or picks meanwhile.
            Phase::Draft { .. } => self.phase = Phase::Next,
            Phase::Ready { map_id, .. } => {
                if s.room.playing {
                    self.phase = Phase::Playing {
                        map_id,
                        results: s.room.results.len(),
                    };
                } else if s.played.contains(&map_id) {
                    self.phase = Phase::Next;
                } else if let Some(entry) = s.entry(map_id) {
                    self.enter_ready(s, entry, out);
                } else {
                    self.phase = Phase::Next;
                }
            }
            Phase::Playing { map_id, results } if !s.room.playing => {
                if s.room.results.len() > results {
                    self.phase = Phase::Scoring {
                        map_id,
                        fetch_at: s.now,
                        tries: 0,
                    };
                } else {
                    self.reopen(s, out);
                }
            }
            Phase::Playing { .. } => {}
            Phase::Scoring { map_id, tries, .. } => {
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now,
                    tries,
                };
            }
        }
    }

    // ---------- for the pages ----------

    pub fn view(&self, s: &Snapshot, enabled: bool) -> AutorefView {
        let (team_id, step, map_id, deadline) = match &self.phase {
            Phase::Waiting => (
                None,
                None,
                None,
                s.starts_at.filter(|&t| s.now < t).or_else(|| s.deadline()),
            ),
            Phase::Warmup { team, until, .. } => {
                (s.teams.get(*team).map(|t| t.id), None, None, Some(*until))
            }
            Phase::Roll { until, .. } => (None, None, None, Some(*until)),
            Phase::Draft { team_id, kind, .. } => (Some(*team_id), Some(*kind), None, None),
            Phase::Ready { map_id, until, .. } => (None, None, Some(*map_id), Some(*until)),
            Phase::Playing { map_id, .. } | Phase::Scoring { map_id, .. } => {
                (None, None, Some(*map_id), None)
            }
            Phase::Next | Phase::Done => (None, None, None, None),
        };
        AutorefView {
            enabled,
            phase: phase_key(&self.phase),
            team_id,
            step,
            map_id,
            deadline,
            teams: s
                .teams
                .iter()
                .map(|t| TeamUsesView {
                    team_id: t.id,
                    aborts_left: self.aborts_left(t.id),
                })
                .collect(),
        }
    }
}

// The Ready phase's fields that change in place.
struct ReadyFields {
    until: OffsetDateTime,
    nf_warned: bool,
    check_at: Option<OffsetDateTime>,
    forced: bool,
    starting: Option<OffsetDateTime>,
}

// Why a team can't ban or pick this map, in the words the room hears.
fn refusal(s: &Snapshot, entry: &PoolEntry, kind: StepKind, team_id: Uuid) -> Option<String> {
    let label = &entry.label;
    if entry.tiebreaker {
        return Some(format!("{label} tiebreaker, {} edilemez.", kind_word(kind)));
    }
    if s.bans.iter().any(|b| b.map_id == entry.map_id) {
        return Some(format!("{label} banlandı, başka bir map seçin."));
    }
    if s.picks.iter().any(|p| p.map_id == entry.map_id) || s.played.contains(&entry.map_id) {
        return Some(format!("{label} zaten oynandı, başka bir map seçin."));
    }
    if kind == StepKind::Pick && !repeatable(&entry.category) {
        let last = s
            .picks
            .iter()
            .filter(|p| p.team_id == team_id)
            .max_by_key(|p| p.created_at)
            .and_then(|p| s.entry(p.map_id));
        if last.is_some_and(|l| l.category == entry.category) {
            return Some(format!(
                "Aynı moddan ({}) arka arkaya pick yapamazsınız (NM hariç), başka bir mod seçin.",
                entry.category
            ));
        }
    }
    None
}

fn phase_key(phase: &Phase) -> &'static str {
    match phase {
        Phase::Waiting => "waiting",
        Phase::Warmup { .. } => "warmup",
        Phase::Roll { .. } => "roll",
        Phase::Draft { .. } => "draft",
        Phase::Ready { .. } => "ready",
        Phase::Playing { .. } => "playing",
        Phase::Scoring { .. } => "scoring",
        Phase::Next => "next",
        Phase::Done => "done",
    }
}

fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Waiting => "oyuncular bekleniyor",
        Phase::Warmup { .. } => "warmup",
        Phase::Roll { .. } => "roll",
        Phase::Draft { .. } => "ban/pick",
        Phase::Ready { .. } => "map hazırlığı",
        Phase::Playing { .. } => "map oynanıyor",
        Phase::Scoring { .. } => "skorlar",
        Phase::Next => "sıradaki adım",
        Phase::Done => "maç bitti",
    }
}

// The auto referee as the referee page shows it.
#[derive(Debug, Clone, Serialize)]
pub struct AutorefView {
    // The match is in auto mode.
    pub enabled: bool,
    pub phase: &'static str,
    // Warmup: the team with the host; draft: the team to ban or pick.
    pub team_id: Option<Uuid>,
    pub step: Option<StepKind>,
    // Ready, playing, scoring: the pool map on.
    pub map_id: Option<Uuid>,
    // When the phase's timer runs out (before the match: its time, then the no-show
    // cutoff).
    #[serde(with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    pub teams: Vec<TeamUsesView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamUsesView {
    pub team_id: Uuid,
    pub aborts_left: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bancho::room::{RoomSlot, SlotTeam};

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
        // At the time, the missing are invited again, once.
        ar.tick(&w.snap(at(1)), &mut out);
        assert!(sent(&out).contains(&"!mp invite #2"));
        out.clear();
        ar.tick(&w.snap(at(2)), &mut out);
        assert!(out.is_empty());
        // Team A completes; B doesn't by the cutoff.
        w.room
            .slots
            .push(slot(2, "a2", 2, SlotStatus::NotReady, ""));
        w.room
            .slots
            .push(slot(3, "b1", 3, SlotStatus::NotReady, ""));
        ar.tick(&w.snap(at(601)), &mut out);
        assert!(out.contains(&Action::Forfeit(2)));
        assert_eq!(ar.phase, Phase::Done);
    }

    #[test]
    fn nobody_complete_hands_over() {
        let mut w = World::bracket();
        w.room.slots = vec![slot(1, "a1", 1, SlotStatus::NotReady, "")];
        let mut ar = AutoRef::default();
        let mut out = Vec::new();
        ar.tick(&w.snap(at(700)), &mut out);
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
        ar.tick(&w.snap(at(601)), &mut out);
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
            ["!mp abort", "Abort kullanıldı. 1 hakkınız kaldı"]
        );
        out.clear();
        ar.player(&w.snap(at(40)), "a1", "-abort", true, &mut out);
        assert_eq!(
            sent(&out),
            ["Maç başladıktan 30 saniye sonra abort kullanılamaz"]
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
        assert!(sent(&out).contains(&"!mp aborttimer"));
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
}
