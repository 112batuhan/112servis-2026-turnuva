//! Reads BanchoBot's lines in a multiplayer channel into the room's state: its slots,
//! teams, map, mods and the scores of every map played. Message formats are
//! BanchoBot's; anything unrecognised is just left in the chat. The room's state is a
//! fold of its saved chat, so a restart reads it back from the database.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::irc::{normalize_name, same_name};

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

// The room id from BanchoBot's reply to !mp make.
pub fn parse_created(text: &str) -> Option<i64> {
    let rest = text.strip_prefix("Created the tournament match ")?;
    let rest = rest.strip_prefix(OSU)?.strip_prefix("/mp/")?;
    let (id, _) = rest.split_once(' ').unwrap_or((rest, ""));
    id.parse().ok()
}

// BanchoBot announcing a player joining: their name, slot and team.
pub fn parse_joined(text: &str) -> Option<(&str, i32, Option<SlotTeam>)> {
    let body = text.strip_suffix('.')?;
    let at = body.rfind(" joined in slot ")?;
    let (name, rest) = (&body[..at], &body[at + " joined in slot ".len()..]);
    if name.is_empty() {
        return None;
    }
    let (slot, team) = match rest.split_once(" for team ") {
        Some((slot, team)) => (
            slot,
            Some(SlotTeam::parse(team).filter(|_| team == "red" || team == "blue")?),
        ),
        None => (rest, None),
    };
    Some((name, digits(slot)?, team))
}

// BanchoBot announcing someone leaving the room, or being kicked from it: their name.
pub fn parse_left(text: &str) -> Option<&str> {
    let name = text.strip_suffix(" left the game.").or_else(|| {
        text.strip_prefix("Kicked ")
            .and_then(|t| t.strip_suffix(" from the match."))
    })?;
    (!name.is_empty()).then_some(name)
}

fn digits(s: &str) -> Option<i32> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

fn word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// BanchoBot's mod names, as osu! abbreviates them.
fn mod_acronym(name: &str) -> String {
    match name {
        "nofail" => "NF",
        "easy" => "EZ",
        "touchdevice" => "TD",
        "hidden" => "HD",
        "hardrock" => "HR",
        "suddendeath" => "SD",
        "doubletime" => "DT",
        "relax" => "RX",
        "halftime" => "HT",
        "nightcore" => "NC",
        "flashlight" => "FL",
        "spunout" => "SO",
        "relax2" | "autopilot" => "AP",
        "perfect" => "PF",
        "fadein" => "FI",
        "mirror" => "MR",
        "scorev2" => "V2",
        other => return other.to_uppercase(),
    }
    .to_string()
}

// "Hidden, HardRock" (BanchoBot's names, freemod left out) as acronyms: "HDHR".
pub fn mod_acronyms(names: &str) -> String {
    names
        .split(',')
        .map(|n| n.split_whitespace().collect::<String>().to_lowercase())
        .filter(|n| !n.is_empty() && n != "freemod" && n != "none")
        .map(|n| mod_acronym(&n))
        .collect()
}

// "HDHR" -> ["HD", "HR"]; "NM" and "" are no mods.
pub fn split_mods(acronyms: &str) -> Vec<String> {
    let upper: Vec<char> = acronyms
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    upper
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| c.iter().collect::<String>())
        .filter(|m| m != "NM")
        .collect()
}

// Whether the room's mods, as BanchoBot described them, include freemod.
pub fn is_freemod(mods: Option<&str>) -> bool {
    let lower = mods.unwrap_or_default().to_lowercase();
    lower.contains("freemod") || lower.contains("free mod")
}

// "Enabled Hidden, NoFail, disabled FreeMod" / "Disabled all mods, enabled FreeMod"
fn describe_mods(text: &str) -> String {
    if text.to_lowercase().contains("enabled freemod") {
        return "Freemod".into();
    }
    match text.strip_prefix("Enabled ") {
        Some(rest) => match rest.find(", disabled ") {
            Some(i) => rest[..i].to_string(),
            None => rest.to_string(),
        },
        None => String::new(),
    }
}

// "Slot 1  Ready https://osu.ppy.sh/u/2 Name  [Host / Team Blue / Hidden]"
fn parse_slot(text: &str) -> Option<RoomSlot> {
    let rest = text.strip_prefix("Slot ")?;
    let (slot, rest) = rest.split_once(' ')?;
    let slot = digits(slot)?;
    let url = format!(" {OSU}/u/");
    let at = rest.find(&url)?;
    let status = rest[..at].trim();
    if !status.starts_with(|c: char| c.is_ascii_alphabetic())
        || !status.chars().all(|c| c.is_ascii_alphabetic() || c == ' ')
    {
        return None;
    }
    let rest = &rest[at + url.len()..];
    let (id, rest) = rest.split_once(' ')?;
    let user_id: i64 = id.parse().ok()?;
    // The name is padded with spaces and may be followed by " [Host / Team Red / mods]".
    let (name, extra) = match rest.strip_suffix(']') {
        Some(inner) => {
            let open = (1..inner.len())
                .filter(|&i| inner.is_char_boundary(i))
                .find(|&i| inner[i..].starts_with(' ') && inner[i..].trim_start().starts_with('['));
            match open {
                Some(i) => {
                    let bracket = inner[i..].find('[').map(|b| i + b + 1)?;
                    (&inner[..i], Some(&inner[bracket..]))
                }
                None => (rest, None),
            }
        }
        None => (rest, None),
    };
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let parts: Vec<&str> = extra
        .unwrap_or_default()
        .split(" / ")
        .map(str::trim)
        .collect();
    let team_part = parts.iter().copied().find(|p| p.starts_with("Team "));
    let mods: Vec<&str> = parts
        .iter()
        .copied()
        .filter(|&p| p != "Host" && Some(p) != team_part && !p.is_empty())
        .collect();
    Some(RoomSlot {
        slot,
        username: name.to_string(),
        user_id: Some(user_id),
        team: team_part.and_then(|t| SlotTeam::parse(&t[5..])),
        status: match status {
            "Ready" => SlotStatus::Ready,
            "No Map" => SlotStatus::NoMap,
            _ => SlotStatus::NotReady,
        },
        host: parts.contains(&"Host"),
        mods: mod_acronyms(&mods.join(", ")),
        mods_known: true,
    })
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

// "Name finished playing (Score: 123, PASSED)."
fn parse_finished(text: &str) -> Option<MapScore> {
    let body = text.strip_suffix(").")?;
    let at = body.rfind(" finished playing (Score: ")?;
    let name = &body[..at];
    let (score, result) = body[at + " finished playing (Score: ".len()..].split_once(", ")?;
    if name.is_empty() || !score.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let passed = match result {
        "PASSED" => true,
        "FAILED" => false,
        _ => return None,
    };
    Some(MapScore {
        username: name.to_string(),
        user_id: None,
        score: score.parse().ok()?,
        passed,
        mods: None,
    })
}

// A room as BanchoBot's lines in its chat describe it.
pub fn read_lobby<'a>(lines: impl IntoIterator<Item = (&'a str, OffsetDateTime)>) -> RoomState {
    let mut room = RoomState::default();
    for (text, at) in lines {
        room.apply(text, at);
    }
    room
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    fn read(lines: &[&str]) -> RoomState {
        read_lobby(lines.iter().map(|l| (*l, now())))
    }

    #[test]
    fn reads_settings() {
        let room = read(&[
            "Room name: 112S: (A) vs (B), History: https://osu.ppy.sh/mp/123",
            "Beatmap: https://osu.ppy.sh/b/42 Artist - Title [Hard]",
            "Team mode: TeamVs, Win condition: ScoreV2",
            "Active mods: Freemod",
            "Players: 2",
            "Slot 1  Not Ready https://osu.ppy.sh/u/-5 Dev Player [Host / Team Blue / Hidden, HardRock]",
            "Slot 3  Ready     https://osu.ppy.sh/u/7 [Toy]           [Team Red]",
        ]);
        assert_eq!(room.name.as_deref(), Some("112S: (A) vs (B)"));
        assert_eq!(room.beatmap_id, Some(42));
        assert_eq!(room.team_mode.as_deref(), Some("TeamVs"));
        assert!(is_freemod(room.mods.as_deref()));
        assert_eq!(room.slots.len(), 2);
        let a = &room.slots[0];
        assert_eq!(
            (a.slot, a.user_id, a.username.as_str()),
            (1, Some(-5), "Dev Player")
        );
        assert_eq!(a.team, Some(SlotTeam::Blue));
        assert!(a.host);
        assert_eq!(a.mods, "HDHR");
        let b = &room.slots[1];
        assert_eq!(
            (b.username.as_str(), b.status, b.team),
            ("[Toy]", SlotStatus::Ready, Some(SlotTeam::Red))
        );
    }

    #[test]
    fn follows_live_events() {
        let mut room = read(&[
            "A joined in slot 1 for team blue.",
            "B b joined in slot 2.",
            "B b moved to slot 4",
            "B b changed to Red",
            "Changed beatmap to https://osu.ppy.sh/b/9 Song [Insane]",
            "Enabled Hidden, NoFail, disabled FreeMod",
            "Added Some_Ref to the match referees",
        ]);
        assert_eq!(
            room.slots.iter().map(|s| s.slot).collect::<Vec<_>>(),
            [1, 4]
        );
        assert_eq!(room.slots[1].team, Some(SlotTeam::Red));
        assert_eq!(room.beatmap_id, Some(9));
        assert_eq!(room.mods.as_deref(), Some("Hidden, NoFail"));
        assert!(room.is_ref("some ref"));
        room.apply(MATCH_STARTED, now());
        assert!(room.playing);
        room.apply("A finished playing (Score: 1000, PASSED).", now());
        room.apply("B b finished playing (Score: 900, FAILED).", now());
        room.apply("The match has finished!", now());
        assert!(!room.playing);
        let result = &room.results[0];
        assert_eq!(result.beatmap_id, Some(9));
        assert_eq!(result.room_mods, "HDNF");
        assert!(!result.freemod);
        assert_eq!(result.scores[1].mods.as_deref(), Some("HDNF"));
        assert!(!result.scores[1].passed);
        room.apply("Kicked A from the match.", now());
        room.apply("B b left the game.", now());
        assert!(room.slots.is_empty());
        room.apply("Removed Some_Ref from the match referees", now());
        assert!(room.refs.is_empty());
    }

    #[test]
    fn parses_replies() {
        assert_eq!(
            parse_created(
                "Created the tournament match https://osu.ppy.sh/mp/111 112S: (A) vs (B)"
            ),
            Some(111)
        );
        assert_eq!(
            parse_joined("x joined in slot 3 for team red."),
            Some(("x", 3, Some(SlotTeam::Red)))
        );
        assert_eq!(parse_left("Kicked y from the match."), Some("y"));
        assert_eq!(mod_acronyms("Hidden, DoubleTime"), "HDDT");
        assert_eq!(split_mods("hdhrNM"), ["HD", "HR"]);
        let mut room = RoomState::default();
        room.apply(
            "Beatmap changed to: Song [Hard] (https://osu.ppy.sh/b/5)",
            now(),
        );
        assert_eq!(room.beatmap_id, Some(5));
        room.apply("Changed match settings to 4 slots, TeamVs, ScoreV2", now());
        assert_eq!(room.size, Some(4));
    }
}
