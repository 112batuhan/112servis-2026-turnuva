//! The auto referee: the room's bot runs the match by itself, from the moment everyone
//! is in the room to the end, the way a referee would. A qualifier lobby plays its pool
//! map by map; a bracket match has its warmups, the roll, the bans and picks the players
//! type in the room, each map started once everyone is ready, the score read from osu!
//! after every map and the match concluded once a team has won. Players steer it with
//! `-abort`, `-durdur` (back to manual), `-rapor`, `-warmup-gec` (and `-davet` in a private message, which
//! the manager answers). It keeps the room locked, so players stay where the bot put them.
//!
//! It is a state machine with no I/O: the manager (manager/auto_referee.rs) feeds it the room's lines,
//! the players' commands and the clock along with a `Snapshot` of the match, and does the
//! `Action`s it returns (lines for the room, bans and picks, a forfeit, a conclusion,
//! notifications). Its state is saved after every change (`match_autoref`), so a restart
//! or a referee switching the match to manual and back continues where it left off
//! (`resume`), going by what the room and the match say happened meanwhile.

mod banchobot;
mod clock;
mod countdown;
mod draft;
mod map;
mod players;
mod resume;
#[cfg(test)]
mod tests;
mod view;
mod warmups;

pub use countdown::{span, Countdown, CountdownKind, GRACE};
pub use view::AutorefView;

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::{
    context::MatchTeam,
    names::same_name,
    room::{is_freemod, mod_acronyms, RoomState},
    tally::{entry_of, PoolEntry, RosterPlayer},
};
use crate::{
    db::{
        autoref::LogLevel,
        bracket::{BanRow, MissedRow, PickRow},
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
// The time a team has to name its ban or pick. A ban that runs out is lost, so it gets
// longer; a pick that runs out passes to the other team.
const BAN_TIME: Duration = Duration::minutes(3);
const PICK_TIME: Duration = Duration::minutes(2);
// The ready timer of a map, in seconds: a qualifier's, a bracket pick's, and the grace a
// player missing NoFail gets once it ran out.
const QUALIFIER_READY: i64 = 180;
const BRACKET_READY: i64 = 120;
const NF_GRACE: i64 = 30;
// The mods a team must have one of on an FM map (on one player at least).
pub const FM_MODS: [&str; 4] = ["HD", "HR", "EZ", "FL"];
// How often the bot asks Bancho again to lock a room it sees unlocked.
const LOCK_EVERY: Duration = Duration::seconds(30);
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
// Said when the match is over; the manager closes the room that long after
// (manager/mod.rs, CLOSE_AFTER_CONCLUDED), and reads the line to know when it went out.
pub const CLOSING: &str = "Maç bitti, oda 30 saniye sonra kapanacak.";

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
    // The team let its ban or pick run out.
    Missed {
        team_id: Uuid,
        kind: StepKind,
    },
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
    // Look the warmup beatmap up on osu! (its length and set); answered with `warmup_map`.
    WarmupMap(i64),
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
    // Bracket: a team is to name its ban or pick by `until`; `candidate` waits for them
    // to say it again.
    Draft {
        team_id: Uuid,
        kind: StepKind,
        candidate: Option<Uuid>,
        #[serde(default = "OffsetDateTime::now_utc", with = "time::serde::rfc3339")]
        until: OffsetDateTime,
    },
    // A map is on, waiting for everyone to be ready or its timer.
    Ready {
        map_id: Uuid,
        #[serde(with = "time::serde::rfc3339")]
        until: OffsetDateTime,
        // The players missing NoFail were told.
        nf_warned: bool,
        // An FM map: the teams with none of HD, HR, EZ, FL were told.
        #[serde(default)]
        fm_warned: bool,
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
    // The wait the room is being counted down through (countdown.rs).
    #[serde(default)]
    pub countdown: Option<Countdown>,
    // When each player last sent -rapor, and -durdur, by osu! id.
    #[serde(skip)]
    reports: HashMap<i64, OffsetDateTime>,
    #[serde(skip)]
    stops: HashMap<i64, OffsetDateTime>,
    // Warmup beatmaps as osu! gave them, by beatmap id.
    #[serde(skip)]
    warmups: HashMap<i64, WarmupMap>,
    // When the bot last asked Bancho to lock the room.
    #[serde(skip)]
    lock_asked: Option<OffsetDateTime>,
}

// A warmup beatmap: its length in seconds and its set, to tell a pool map's other
// difficulties.
#[derive(Debug, Clone, Copy)]
pub struct WarmupMap {
    pub seconds: i32,
    pub beatmapset_id: i64,
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
    // Bracket: the bans and picks that ran out, in the order they did.
    pub missed: &'a [MissedRow],
    // Bracket: the next ban or pick.
    pub turn: Option<Turn>,
    // Bracket: the map wins so far.
    pub score: Option<(i32, i32)>,
    pub best_of: Option<i32>,
    // Pool maps finished in any room of the match and counted.
    pub played: &'a HashSet<Uuid>,
    // Pool maps finished in any room, counted or left out ("Sayma").
    pub finished: &'a HashSet<Uuid>,
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

    // The team whose pick just ran out and passed to the other team: the latest turn
    // taken was that missed pick.
    fn passed_from(&self) -> Option<Uuid> {
        let last = self.missed.last().filter(|m| m.kind == StepKind::Pick)?;
        let latest = self.bans.iter().all(|b| b.created_at < last.created_at)
            && self.picks.iter().all(|p| p.created_at < last.created_at);
        latest.then_some(last.team_id)
    }

    // The pool map on in the room, by its beatmap (and, for a beatmap the pool has twice,
    // the mods it's on with).
    fn room_entry(&self) -> Option<&PoolEntry> {
        let mods = self.room.mods.as_deref();
        let picked: HashSet<Uuid> = self.picks.iter().map(|p| p.map_id).collect();
        entry_of(
            self.entries,
            self.room.beatmap_id,
            &mod_acronyms(mods.unwrap_or_default()),
            is_freemod(mods),
            &picked,
        )
    }

    // Whether the beatmap is one of the pool's songs: a pool map, or another difficulty
    // of one (`set`, when osu! said it).
    fn in_pool(&self, beatmap: i64, set: Option<i64>) -> bool {
        self.entries
            .iter()
            .any(|e| e.beatmap_id == beatmap || Some(e.beatmapset_id) == set)
    }

    // Whether the match got past its start without the bot: a map played, a ban, a pick
    // or a missed turn, or the first turn decided (a roll the referee settled).
    fn started(&self) -> bool {
        !self.finished.is_empty()
            || !self.bans.is_empty()
            || !self.picks.is_empty()
            || !self.missed.is_empty()
            || self.turn.is_some_and(|t| t.team_id.is_some())
    }

    // The team (0 or 1) of the room's host.
    fn host_team(&self) -> Option<usize> {
        let host = self.room.slots.iter().find(|s| s.host)?;
        let p = self.roster.iter().find(|p| match host.user_id {
            Some(id) => id == p.osu_id,
            None => same_name(&host.username, &p.username),
        })?;
        self.teams.iter().position(|t| t.id == p.team_id)
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

// The command a player wrote, lowercased, and what follows it.
fn command(text: &str) -> Option<(String, &str)> {
    let text = text.trim();
    if !text.starts_with('-') {
        return None;
    }
    let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    Some((word.to_lowercase().replace('ç', "c"), rest.trim()))
}

// The Ready phase's fields that change in place.
struct ReadyFields {
    until: OffsetDateTime,
    nf_warned: bool,
    fm_warned: bool,
    check_at: Option<OffsetDateTime>,
    forced: bool,
    starting: Option<OffsetDateTime>,
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
}
