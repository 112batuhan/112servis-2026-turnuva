//! BanchoBot's lines, word by word.

use super::mods::mod_acronyms;
use super::{MapScore, RoomSlot, SlotStatus, SlotTeam, OSU};

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

pub(super) fn digits(s: &str) -> Option<i32> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

pub(super) fn word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// "Slot 1  Ready https://osu.ppy.sh/u/2 Name  [Host / Team Blue / Hidden]"
pub(super) fn parse_slot(text: &str) -> Option<RoomSlot> {
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

// "Name finished playing (Score: 123, PASSED)."
pub(super) fn parse_finished(text: &str) -> Option<MapScore> {
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
