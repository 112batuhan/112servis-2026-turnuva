//! Reads BanchoBot's lines in a multiplayer channel into the room's state: its slots,
//! teams, map, mods and the scores of every map played. Message formats are
//! BanchoBot's; anything unrecognised is just left in the chat. The room's state is a
//! fold of its saved chat, so a restart reads it back from the database.

pub mod commands;
mod mods;
mod parse;
#[cfg(test)]
mod tests;

pub use mods::{is_freemod, mod_acronyms, split_mods};
pub use parse::{parse_created, parse_joined, parse_left};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::bancho::names::{normalize_name, same_name};

use self::{
    mods::describe_mods,
    parse::{digits, parse_finished, parse_slot, word},
};

// BanchoBot's line as a map starts.
pub const MATCH_STARTED: &str = "The match has started!";

const OSU: &str = "https://osu.ppy.sh";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotTeam {
    Red,
    Blue,
}

impl SlotTeam {
    pub fn as_str(self) -> &'static str {
        match self {
            SlotTeam::Red => "red",
            SlotTeam::Blue => "blue",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "red" => Some(SlotTeam::Red),
            "blue" => Some(SlotTeam::Blue),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotStatus {
    Ready,
    NotReady,
    NoMap,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoomSlot {
    // 1-16.
    pub slot: i32,
    pub username: String,
    // Known from !mp settings; joins only give the name.
    pub user_id: Option<i64>,
    pub team: Option<SlotTeam>,
    pub status: SlotStatus,
    pub host: bool,
    // Mods the player picked, with freemod on, as acronyms ("HDHR"); from !mp settings.
    pub mods: String,
    // Whether `mods` came from !mp settings; a player who just joined hasn't been read.
    pub mods_known: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapScore {
    pub username: String,
    // Known when the score came from osu! rather than the chat, which only names players.
    pub user_id: Option<i64>,
    pub score: i64,
    pub passed: bool,
    // The mods the player played with, the room's included; None when unknown.
    pub mods: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapResult {
    pub beatmap_id: Option<i64>,
    pub beatmap_name: Option<String>,
    // The room's own mods as acronyms ("HRNF"), without freemod.
    pub room_mods: String,
    pub freemod: bool,
    pub scores: Vec<MapScore>,
    // Each player's team when the map finished, by normalize_name.
    #[serde(skip)]
    pub teams: HashMap<String, Option<SlotTeam>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RoomState {
    pub name: Option<String>,
    pub beatmap_id: Option<i64>,
    pub beatmap_name: Option<String>,
    pub team_mode: Option<String>,
    pub win_condition: Option<String>,
    // As BanchoBot last described them: "HardRock, NoFail", "Freemod" or "".
    pub mods: Option<String>,
    pub size: Option<i32>,
    pub locked: Option<bool>,
    pub playing: bool,
    pub closed: bool,
    pub slots: Vec<RoomSlot>,
    // Scores of the map being played.
    pub current: Vec<MapScore>,
    // Finished maps, oldest first.
    #[serde(skip)]
    pub results: Vec<MapResult>,
    // Who was added as a referee of the room and not removed since, by normalize_name.
    // The room's maker isn't listed.
    pub refs: Vec<String>,
    // When the map being played started: the room's last "The match has started!".
    #[serde(with = "time::serde::rfc3339::option")]
    pub map_started_at: Option<OffsetDateTime>,
}

impl RoomState {
    fn sort_slots(&mut self) {
        self.slots.sort_by_key(|s| s.slot);
    }

    fn slot_of(&mut self, name: &str) -> Option<&mut RoomSlot> {
        self.slots.iter_mut().find(|s| same_name(&s.username, name))
    }

    // Bancho unreadies everyone when the map changes.
    fn change_beatmap(&mut self, beatmap_id: i64, name: &str) {
        self.beatmap_id = Some(beatmap_id);
        self.beatmap_name = Some(name.to_string());
        for s in &mut self.slots {
            s.status = SlotStatus::NotReady;
        }
    }

    // Whether someone was added as a referee of the room.
    pub fn is_ref(&self, username: &str) -> bool {
        let name = normalize_name(username);
        self.refs.contains(&name)
    }

    // Applies one BanchoBot line from the room's channel, said at `at`.
    pub fn apply(&mut self, text: &str, at: OffsetDateTime) {
        // !mp settings: a snapshot, so the slot list starts over at "Players: N".
        if let Some(rest) = text.strip_prefix("Room name: ") {
            if let Some(i) = rest.rfind(&format!(", History: {OSU}/mp/")) {
                self.name = Some(rest[..i].to_string());
                return;
            }
        }
        if let Some(rest) = text.strip_prefix(&format!("Beatmap: {OSU}/b/")) {
            if let Some((id, name)) = rest.split_once(' ') {
                if let Ok(id) = id.parse() {
                    self.beatmap_id = Some(id);
                    self.beatmap_name = Some(name.to_string());
                    return;
                }
            }
        }
        if let Some(rest) = text.strip_prefix("Team mode: ") {
            if let Some((mode, win)) = rest.split_once(", Win condition: ") {
                if word(mode) && word(win) {
                    self.team_mode = Some(mode.to_string());
                    self.win_condition = Some(win.to_string());
                    return;
                }
            }
        }
        if let Some(mods) = text.strip_prefix("Active mods: ") {
            self.mods = Some(mods.to_string());
            return;
        }
        if text.strip_prefix("Players: ").and_then(digits).is_some() {
            self.slots.clear();
            return;
        }
        if let Some(entry) = parse_slot(text) {
            self.slots
                .retain(|s| s.slot != entry.slot && s.user_id != entry.user_id);
            self.slots.push(entry);
            self.sort_slots();
            return;
        }

        // Live events.
        if let Some((name, slot, team)) = parse_joined(text) {
            let known = self
                .slots
                .iter()
                .find(|s| same_name(&s.username, name))
                .and_then(|s| s.user_id);
            self.slots
                .retain(|s| s.slot != slot && !same_name(&s.username, name));
            self.slots.push(RoomSlot {
                slot,
                username: name.to_string(),
                user_id: known,
                team,
                status: SlotStatus::NotReady,
                host: false,
                mods: String::new(),
                mods_known: false,
            });
            self.sort_slots();
            return;
        }
        if let Some(at_) = text.rfind(" moved to slot ") {
            if let Some(slot) = digits(&text[at_ + " moved to slot ".len()..]) {
                let name = &text[..at_];
                if let Some(s) = self.slot_of(name) {
                    s.slot = slot;
                }
                self.sort_slots();
                return;
            }
        }
        if let Some(at_) = text.rfind(" changed to ") {
            let team = &text[at_ + " changed to ".len()..];
            if team == "Red" || team == "Blue" {
                let name = text[..at_].to_string();
                let team = SlotTeam::parse(team);
                if let Some(s) = self.slot_of(&name) {
                    s.team = team;
                }
                return;
            }
        }
        if let Some(name) = parse_left(text) {
            let name = name.to_string();
            self.slots.retain(|s| !same_name(&s.username, &name));
            return;
        }
        if let Some(name) = text
            .strip_suffix(" became the host.")
            .or_else(|| text.strip_prefix("Changed match host to "))
        {
            let name = name.to_string();
            for s in &mut self.slots {
                s.host = same_name(&s.username, &name);
            }
            return;
        }
        if text == "Cleared match host" {
            for s in &mut self.slots {
                s.host = false;
            }
            return;
        }
        if let Some(rest) = text.strip_prefix(&format!("Changed beatmap to {OSU}/b/")) {
            if let Some((id, name)) = rest.split_once(' ') {
                if let Ok(id) = id.parse() {
                    self.change_beatmap(id, name);
                    return;
                }
            }
        }
        if let Some(rest) = text.strip_prefix("Beatmap changed to: ") {
            let tail = format!(" ({OSU}/b/");
            if let Some(i) = rest.rfind(&tail) {
                if let Some(id) = rest[i + tail.len()..]
                    .strip_suffix(')')
                    .and_then(|id| id.parse().ok())
                {
                    let name = rest[..i].to_string();
                    self.change_beatmap(id, &name);
                    return;
                }
            }
        }
        if let Some(rest) = text.strip_prefix("Changed match settings to ") {
            let (size, rest) = match rest.split_once(" slots, ") {
                Some((n, rest)) if digits(n).is_some() => (digits(n), rest),
                _ => (None, rest),
            };
            if let Some((mode, win)) = rest.split_once(", ") {
                if word(mode) && word(win) {
                    self.size = size.or(self.size);
                    self.team_mode = Some(mode.to_string());
                    self.win_condition = Some(win.to_string());
                    return;
                }
            }
        }
        if let Some(size) = text.strip_prefix("Changed match to size ").and_then(digits) {
            self.size = Some(size);
            return;
        }
        if text.starts_with("Enabled ") || text.starts_with("Disabled ") {
            let mods = describe_mods(text);
            // Without freemod, osu! takes the players' own mods off.
            if !is_freemod(Some(&mods)) {
                for s in &mut self.slots {
                    s.mods.clear();
                }
            }
            self.mods = Some(mods);
            return;
        }
        if let Some(name) = text
            .strip_prefix("Added ")
            .and_then(|t| t.strip_suffix(" to the match referees"))
        {
            let name = normalize_name(name);
            if !self.refs.contains(&name) {
                self.refs.push(name);
            }
            return;
        }
        if let Some(name) = text
            .strip_prefix("Removed ")
            .and_then(|t| t.strip_suffix(" from the match referees"))
        {
            let name = normalize_name(name);
            self.refs.retain(|r| *r != name);
            return;
        }
        match text {
            "Locked the match" => self.locked = Some(true),
            "Unlocked the match" => self.locked = Some(false),
            "All players are ready" => {
                for s in &mut self.slots {
                    s.status = SlotStatus::Ready;
                }
            }
            MATCH_STARTED => {
                self.playing = true;
                self.current.clear();
                self.map_started_at = Some(at);
            }
            "The match has finished!" => self.finish(),
            "Aborted the match" => {
                self.playing = false;
                self.current.clear();
                self.map_started_at = None;
            }
            "Closed the match" => {
                self.closed = true;
                self.playing = false;
                self.map_started_at = None;
            }
            _ => {
                if let Some(score) = parse_finished(text) {
                    self.current
                        .retain(|s| !same_name(&s.username, &score.username));
                    self.current.push(score);
                }
            }
        }
    }

    fn finish(&mut self) {
        let teams = self
            .slots
            .iter()
            .map(|s| (normalize_name(&s.username), s.team))
            .collect();
        let freemod = is_freemod(self.mods.as_deref());
        let room_mods = mod_acronyms(self.mods.as_deref().unwrap_or_default());
        // Each player's mods: the room's, plus their own with freemod as the last
        // !mp settings showed them (the bot asks for it as each map starts).
        let scores = std::mem::take(&mut self.current)
            .into_iter()
            .map(|score| {
                let slot = self
                    .slots
                    .iter()
                    .find(|s| same_name(&s.username, &score.username));
                let own = if freemod {
                    slot.filter(|s| s.mods_known).map(|s| s.mods.clone())
                } else {
                    Some(String::new())
                };
                MapScore {
                    user_id: score.user_id.or(slot.and_then(|s| s.user_id)),
                    mods: own.map(|own| format!("{room_mods}{own}")),
                    ..score
                }
            })
            .collect();
        self.results.push(MapResult {
            beatmap_id: self.beatmap_id,
            beatmap_name: self.beatmap_name.clone(),
            room_mods,
            freemod,
            scores,
            teams,
        });
        self.playing = false;
        self.map_started_at = None;
        for s in &mut self.slots {
            s.status = SlotStatus::NotReady;
        }
    }
}

// A room as BanchoBot's lines in its chat describe it.
pub fn read_lobby<'a>(lines: impl IntoIterator<Item = (&'a str, OffsetDateTime)>) -> RoomState {
    let mut room = RoomState::default();
    for (text, at) in lines {
        room.apply(text, at);
    }
    room
}
