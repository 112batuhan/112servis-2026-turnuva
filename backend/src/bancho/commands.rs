//! The !mp commands the bots send: setting a room up, placing players, maps and mods.

use std::collections::HashSet;

use super::{
    irc::{irc_name, same_name},
    room::{RoomSlot, RoomState, SlotTeam},
};

// The most slots an osu! room has.
const MAX_SLOTS: i32 = 16;

// !mp set's team modes.
pub const HEAD_TO_HEAD: i32 = 0;
pub const TEAM_VS: i32 = 2;

// Prefixed to every room name: "112S: (Team A) vs (Team B)".
pub const ROOM_PREFIX: &str = "112S";

// The colour each side of a bracket match plays as: team 1 blue, team 2 red.
pub fn side_color(side: u8) -> SlotTeam {
    if side == 1 {
        SlotTeam::Blue
    } else {
        SlotTeam::Red
    }
}

// The slots each side's two players sit in: team 1 (blue) in 1 and 2, team 2 in 3 and 4.
pub fn side_slots(side: u8) -> [i32; 2] {
    if side == 1 {
        [1, 2]
    } else {
        [3, 4]
    }
}

// "#<osu id>" addresses a player in !mp commands whatever their current name.
pub fn player_ref(osu_id: i64) -> String {
    format!("#{osu_id}")
}

// A player in the room, by osu! id when it's known and as BanchoBot names them otherwise.
pub fn slot_ref(slot: &RoomSlot) -> String {
    match slot.user_id {
        Some(id) => player_ref(id),
        None => irc_name(&slot.username),
    }
}

// How many slots the room has, as far as anyone can tell.
pub fn room_size(room: &RoomState, at_least: i32) -> i32 {
    let biggest = room.slots.iter().map(|s| s.slot).max().unwrap_or(0);
    MAX_SLOTS.min(room.size.unwrap_or(0).max(at_least).max(biggest))
}

// Who belongs in a bracket match's slots, for moving anyone else out of them.
pub struct MatchSlots<'a> {
    pub reserved: [i32; 4],
    pub players: &'a [Placement],
}

impl MatchSlots<'_> {
    fn is_player(&self, slot: &RoomSlot) -> bool {
        self.players
            .iter()
            .any(|p| slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username))
    }
}

pub fn match_slots(players: &[Placement]) -> MatchSlots<'_> {
    MatchSlots {
        reserved: [1, 2, 3, 4],
        players,
    }
}

// Where a player belongs in the room.
#[derive(Debug, Clone)]
pub struct Placement {
    pub osu_id: i64,
    pub username: String,
    // Their slot; None when any will do.
    pub slot: Option<i32>,
    // Their colour; None in head to head.
    pub team: Option<SlotTeam>,
}

// The commands that put a player in the room into slot `to` and onto `team` (either None
// to leave it as it is). A slot someone else holds is swapped: they wait in a free slot
// (the room grows by one for them when it's full, and shrinks back after), the player
// takes the slot, and they take the player's old one. Given `matched`, someone in a match
// slot who isn't one of its players is moved out to a free slot past the match's instead,
// and stays there.
pub fn move_commands(
    room: &RoomState,
    mover: &RoomSlot,
    to: Option<i32>,
    team: Option<SlotTeam>,
    matched: Option<&MatchSlots>,
) -> Vec<String> {
    let mut commands = Vec::new();
    if let Some(to) = to.filter(|&to| to != mover.slot) {
        let other = room
            .slots
            .iter()
            .find(|s| s.slot == to && !same_name(&s.username, &mover.username));
        match other {
            None => commands.push(format!("!mp move {} {to}", slot_ref(mover))),
            Some(other) => {
                let outsider = matched.filter(|m| !m.is_player(other));
                // A room grown for an outsider grows past the match's slots.
                let at_least = match outsider {
                    Some(m) => to.max(*m.reserved.iter().max().unwrap_or(&0)),
                    None => to,
                };
                let size = room_size(room, at_least);
                let taken: HashSet<i32> = room.slots.iter().map(|s| s.slot).collect();
                let mut spare = (1..=size).find(|n| {
                    !taken.contains(n) && !outsider.is_some_and(|m| m.reserved.contains(n))
                });
                let grow = spare.is_none() && size < MAX_SLOTS;
                if grow {
                    spare = Some(size + 1);
                    commands.push(format!("!mp size {}", size + 1));
                }
                if let Some(spare) = spare {
                    commands.push(format!("!mp move {} {spare}", slot_ref(other)));
                    commands.push(format!("!mp move {} {to}", slot_ref(mover)));
                    if outsider.is_none() {
                        commands.push(format!("!mp move {} {}", slot_ref(other), mover.slot));
                        if grow {
                            commands.push(format!("!mp size {size}"));
                        }
                    }
                }
            }
        }
    }
    if let Some(team) = team.filter(|&t| Some(t) != mover.team) {
        commands.push(format!("!mp team {} {}", slot_ref(mover), team.as_str()));
    }
    commands
}

// The room after the !mp move, team and size commands move_commands makes.
fn after_commands(room: &mut RoomState, commands: &[String]) {
    for line in commands {
        let parts: Vec<&str> = line.split(' ').collect();
        match parts.as_slice() {
            [_, "size", n] => room.size = n.parse().ok(),
            [_, verb, r, value] => {
                for s in room.slots.iter_mut().filter(|s| slot_ref(s) == *r) {
                    match *verb {
                        "move" => s.slot = value.parse().unwrap_or(s.slot),
                        "team" => {
                            s.team = Some(if *value == "red" {
                                SlotTeam::Red
                            } else {
                                SlotTeam::Blue
                            })
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

// The commands that put every one of `players` who's in the room in their place, one
// after the other, each move counted in for the next.
pub fn place_commands(
    room: &RoomState,
    players: &[Placement],
    matched: Option<&MatchSlots>,
) -> Vec<String> {
    let mut state = room.clone();
    // Players are addressed by osu! id, which the chat's joins don't give.
    for s in &mut state.slots {
        if let Some(p) = players
            .iter()
            .find(|p| s.user_id == Some(p.osu_id) || same_name(&s.username, &p.username))
        {
            s.user_id = Some(p.osu_id);
        }
    }
    let mut commands = Vec::new();
    for p in players {
        let Some(mover) = state
            .slots
            .iter()
            .find(|s| s.user_id == Some(p.osu_id))
            .cloned()
        else {
            continue;
        };
        let next = move_commands(&state, &mover, p.slot, p.team, matched);
        after_commands(&mut state, &next);
        commands.extend(next);
    }
    commands
}

// A pool category is freemod when it's named FM or TB (FM1, TB, …).
pub fn is_freemod_category(name: &str) -> bool {
    let n = name.trim().to_uppercase();
    n.starts_with("FM") || n.starts_with("TB")
}

pub fn is_tiebreaker_category(name: &str) -> bool {
    name.trim().to_uppercase().starts_with("TB")
}

// The !mp mods line for a pool map. "HDDT" -> "!mp mods HD DT NF". Freemod maps get just
// "Freemod": players pick their own mods there, NoFail included.
pub fn mods_command(mods: &str, freemod: bool, no_fail: bool) -> String {
    let mut list: Vec<String> = if freemod {
        vec!["Freemod".into()]
    } else {
        let upper: Vec<char> = mods
            .to_uppercase()
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect();
        upper
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| c.iter().collect::<String>())
            .filter(|m| m != "NM")
            .collect()
    };
    if no_fail && !freemod && !list.iter().any(|m| m == "NF") {
        list.push("NF".into());
    }
    if list.is_empty() {
        "!mp mods None".into()
    } else {
        format!("!mp mods {}", list.join(" "))
    }
}

// The !mp map line for a pool map: its beatmap in its game mode.
pub fn map_command(beatmap_id: i64, mode: &str) -> String {
    let mode = match mode {
        "taiko" => 1,
        "fruits" => 2,
        "mania" => 3,
        _ => 0,
    };
    format!("!mp map {beatmap_id} {mode}")
}

pub fn kick_command(slot: &RoomSlot) -> String {
    format!("!mp kick {}", slot_ref(slot))
}

// Makes someone a referee of the room, so they can join its channel and their tournament
// client shows its chat. By name, as !mp addref takes it.
pub fn add_ref_command(username: &str) -> String {
    format!("!mp addref {}", irc_name(username))
}

pub fn remove_ref_command(username: &str) -> String {
    format!("!mp removeref {}", irc_name(username))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bancho::room::SlotStatus;

    fn slot(n: i32, name: &str, id: Option<i64>, team: Option<SlotTeam>) -> RoomSlot {
        RoomSlot {
            slot: n,
            username: name.into(),
            user_id: id,
            team,
            status: SlotStatus::NotReady,
            host: false,
            mods: String::new(),
            mods_known: false,
        }
    }

    fn placement(id: i64, name: &str, n: i32, team: SlotTeam) -> Placement {
        Placement {
            osu_id: id,
            username: name.into(),
            slot: Some(n),
            team: Some(team),
        }
    }

    #[test]
    fn moves_to_a_free_slot() {
        let room = RoomState {
            size: Some(4),
            slots: vec![slot(3, "a", Some(1), None)],
            ..Default::default()
        };
        let cmds = move_commands(&room, &room.slots[0], Some(1), Some(SlotTeam::Blue), None);
        assert_eq!(cmds, ["!mp move #1 1", "!mp team #1 blue"]);
    }

    #[test]
    fn swaps_through_a_spare_slot() {
        let room = RoomState {
            size: Some(2),
            slots: vec![slot(1, "b", Some(2), None), slot(2, "a", Some(1), None)],
            ..Default::default()
        };
        let cmds = move_commands(&room, &room.slots[1], Some(1), None, None);
        assert_eq!(
            cmds,
            [
                "!mp size 3",
                "!mp move #2 3",
                "!mp move #1 1",
                "!mp move #2 2",
                "!mp size 2"
            ]
        );
    }

    #[test]
    fn moves_outsiders_past_the_match() {
        let players = [placement(1, "a", 1, SlotTeam::Blue)];
        let matched = match_slots(&players);
        let room = RoomState {
            size: Some(5),
            slots: vec![
                slot(1, "stranger", Some(9), None),
                slot(2, "a", Some(1), Some(SlotTeam::Blue)),
            ],
            ..Default::default()
        };
        let cmds = move_commands(
            &room,
            &room.slots[1],
            Some(1),
            Some(SlotTeam::Blue),
            Some(&matched),
        );
        assert_eq!(cmds, ["!mp move #9 5", "!mp move #1 1"]);
    }

    #[test]
    fn places_everyone_in_turn() {
        let players = [
            placement(1, "a", 1, SlotTeam::Blue),
            placement(2, "b", 3, SlotTeam::Red),
        ];
        let room = RoomState {
            size: Some(4),
            slots: vec![slot(1, "b", None, None), slot(3, "a", None, None)],
            ..Default::default()
        };
        let cmds = place_commands(&room, &players, Some(&match_slots(&players)));
        assert_eq!(
            cmds,
            [
                "!mp move #2 2",
                "!mp move #1 1",
                "!mp move #2 3",
                "!mp team #1 blue",
                "!mp team #2 red"
            ]
        );
    }

    #[test]
    fn builds_mods() {
        assert_eq!(mods_command("HDDT", false, true), "!mp mods HD DT NF");
        assert_eq!(mods_command("NM", false, false), "!mp mods None");
        assert_eq!(mods_command("", true, true), "!mp mods Freemod");
        assert_eq!(map_command(5, "osu"), "!mp map 5 0");
    }
}
