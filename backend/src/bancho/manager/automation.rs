//! What a room needs doing by itself as its lines come in: placing the players,
//! adding referees and the staff, kicking strangers.

use std::collections::HashSet;

use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    connection::{bot::Priority, irc::room_channel},
    context::MatchContext,
    names::{normalize_name, same_name},
    room::commands::{
        add_ref_command, kick_command, match_slots, move_commands, place_commands, Placement,
    },
    room::{parse_joined, parse_left, RoomSlot, RoomState, MATCH_STARTED},
};
use crate::{db::user::StaffMember, role::Role};

use super::{Manager, REPOSITION_EVERY, REPOSITION_SETTLE, REPOSITION_TRIES};

impl Manager {
    // Once all of a bracket match's players are in its room, anyone still out of their
    // slot or colour is moved again. Each join's moves are worked out from the room as it
    // was then, so when several join at once a move can find its slot taken meanwhile, and
    // nothing else would try it again.
    pub(super) fn reposition(&mut self, now: Instant) {
        let due: Vec<i64> = self
            .rooms
            .values()
            .filter(|r| r.reposition_at.is_some_and(|t| t <= now))
            .map(|r| r.mp_id)
            .collect();
        for mp_id in due {
            let next = self.reposition_room(mp_id, now);
            if let Some(room) = self.rooms.get_mut(&mp_id) {
                room.reposition_at = next;
            }
        }
    }

    // Moves the room's players if they're all in and someone is out of place; when to look
    // again.
    fn reposition_room(&mut self, mp_id: i64, now: Instant) -> Option<Instant> {
        let room = self.rooms.get(&mp_id)?;
        let bot = room.bot.filter(|_| room.joined && !room.closed)?;
        let ctx = &self.runs.get(&room.match_id)?.ctx;
        if self.latest_room(room.match_id) != Some(mp_id)
            || !ctx.is_bracket()
            || !ctx.settings.auto_place
            || !self.bot_online(Some(bot))
            || room.reposition_tries >= REPOSITION_TRIES
        {
            return None;
        }
        // Not during a map, nor while the bot's earlier lines are still on their way.
        let queued = self
            .bots
            .get(&bot)
            .and_then(|b| b.conn.as_ref())
            .map_or(0, |(_, conn)| conn.stats.lock().unwrap().queued);
        if room.state.playing || queued > 0 {
            return Some(now + REPOSITION_SETTLE);
        }
        let placements = Self::placements(ctx);
        let everyone_in = placements.iter().all(|p| {
            room.state
                .slots
                .iter()
                .any(|s| s.user_id == Some(p.osu_id) || same_name(&s.username, &p.username))
        });
        if !everyone_in {
            return None;
        }
        let mut lines = place_commands(&room.state, &placements, Some(&match_slots(&placements)));
        if lines.is_empty() {
            return None;
        }
        lines.push("!mp settings".into());
        let tries = room.reposition_tries + 1;
        for line in lines {
            self.bot_send(
                bot,
                room_channel(mp_id),
                line,
                Some(mp_id),
                Priority::Normal,
            );
        }
        let room = self.rooms.get_mut(&mp_id)?;
        room.reposition_tries = tries;
        (tries < REPOSITION_TRIES).then_some(now + REPOSITION_EVERY)
    }

    pub(super) fn placements(ctx: &MatchContext) -> Vec<Placement> {
        ctx.roster
            .iter()
            .filter(|r| r.color.is_some())
            .map(|r| Placement {
                osu_id: r.osu_id,
                username: r.username.clone(),
                slot: r.home,
                team: r.color,
            })
            .collect()
    }

    // Staff who may be in a room without playing.
    fn staff_of(&self, slot: &RoomSlot) -> Option<&StaffMember> {
        self.staff.iter().find(|s| {
            [Role::Host, Role::Referee, Role::Streamer]
                .iter()
                .any(|r| s.roles.contains(r))
                && match slot.user_id {
                    Some(id) => s.osu_id == id,
                    None => same_name(&s.username, &slot.username),
                }
        })
    }

    pub(super) fn is_member(&self, ctx: &MatchContext, slot: &RoomSlot) -> bool {
        ctx.roster.iter().any(|p| match slot.user_id {
            Some(id) => p.osu_id == id,
            None => same_name(&p.username, &slot.username),
        }) || self.staff_of(slot).is_some()
    }

    // What the room does by itself as BanchoBot reports it: reads everyone's mods as each
    // map starts, puts bracket players in their slot and colour as they join, makes every
    // holder of Referee who joins one of its referees (never a bot), and kicks whoever
    // joins without belonging there. Only in the match's latest room.
    pub(super) fn automate(&mut self, match_id: Uuid, mp_id: i64, text: &str) {
        if self.latest_room(match_id) != Some(mp_id) {
            return;
        }
        let Some(bot) = self
            .rooms
            .get(&mp_id)
            .filter(|r| r.joined && !r.closed)
            .and_then(|r| r.bot)
        else {
            return;
        };
        if !self.bot_online(Some(bot)) {
            return;
        }
        let channel = room_channel(mp_id);
        if text == MATCH_STARTED {
            self.bot_send(
                bot,
                channel,
                "!mp settings".into(),
                Some(mp_id),
                Priority::Low,
            );
            return;
        }
        let ctx = self.runs[&match_id].ctx.clone();
        let mut out: Vec<String> = Vec::new();
        let room = &self.rooms[&mp_id];

        // !mp settings gave the ids of those who joined under an unknown name.
        let mut unknown = room.unknown.clone();
        let mut settle: Vec<RoomSlot> = Vec::new();
        if !unknown.is_empty() {
            for slot in &room.state.slots {
                let name = normalize_name(&slot.username);
                if slot.user_id.is_some() && unknown.remove(&name) {
                    settle.push(slot.clone());
                }
            }
            if let Some(left) = parse_left(text) {
                unknown.remove(&normalize_name(left));
            }
        }
        let mut refs_asked = room.refs_asked.clone();
        for slot in &settle {
            if let Some(p) = ctx.roster.iter().find(|p| Some(p.osu_id) == slot.user_id) {
                out.extend(self.place(&ctx, &room.state, slot, p.osu_id));
            } else if let Some(line) = self.add_ref(&room.state, slot, &mut refs_asked) {
                out.extend(line);
            } else if ctx.settings.auto_kick && self.staff_loaded && !self.is_member(&ctx, slot) {
                out.push(kick_command(slot));
            }
        }

        if let Some((name, _, _)) = parse_joined(text) {
            if let Some(slot) = room
                .state
                .slots
                .iter()
                .find(|s| same_name(&s.username, name))
                .cloned()
            {
                if let Some(p) = ctx.roster.iter().find(|p| same_name(&p.username, name)) {
                    out.extend(self.place(&ctx, &room.state, &slot, p.osu_id));
                } else if let Some(line) = self.add_ref(&room.state, &slot, &mut refs_asked) {
                    out.extend(line);
                } else if !self.is_member(&ctx, &slot)
                    && (ctx.settings.auto_kick || ctx.settings.auto_place)
                    && !self.is_bot_name(name)
                {
                    // A name no member has: a stranger, or someone who renamed on osu! since
                    // the site last saw them. Settled by the osu! id !mp settings gives.
                    unknown.insert(normalize_name(name));
                    out.push("!mp settings".into());
                }
            }
        }
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        room.unknown = unknown;
        room.refs_asked = refs_asked;
        if parse_joined(text).is_some() {
            room.reposition_at = Some(Instant::now() + REPOSITION_SETTLE);
            room.reposition_tries = 0;
        }
        for line in out {
            self.bot_send(bot, channel.clone(), line, Some(mp_id), Priority::Normal);
        }
    }

    // A bracket player to their slot and colour; Bancho barely answers !mp team, so the
    // room is read again after.
    fn place(
        &self,
        ctx: &MatchContext,
        state: &RoomState,
        slot: &RoomSlot,
        osu_id: i64,
    ) -> Vec<String> {
        if !ctx.is_bracket() || !ctx.settings.auto_place {
            return Vec::new();
        }
        let placements = Self::placements(ctx);
        let Some(p) = placements.iter().find(|p| p.osu_id == osu_id) else {
            return Vec::new();
        };
        let mover = RoomSlot {
            user_id: Some(osu_id),
            ..slot.clone()
        };
        let mut lines = move_commands(
            state,
            &mover,
            p.slot,
            p.team,
            Some(&match_slots(&placements)),
        );
        if !lines.is_empty() {
            lines.push("!mp settings".into());
        }
        lines
    }

    // A holder of Referee who joined the room is made one of its referees, once; by the
    // name the join gave. Some(lines) when they're a referee.
    fn add_ref(
        &self,
        state: &RoomState,
        slot: &RoomSlot,
        asked: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        let member = self
            .staff_of(slot)
            .filter(|s| s.roles.contains(&Role::Referee))?;
        if self.is_bot_name(&member.username) {
            return None;
        }
        let name = normalize_name(&slot.username);
        if state.is_ref(&slot.username) || !asked.insert(name) {
            return Some(Vec::new());
        }
        Some(vec![add_ref_command(&slot.username)])
    }
}
