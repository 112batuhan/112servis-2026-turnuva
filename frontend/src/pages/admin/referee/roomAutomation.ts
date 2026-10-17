import { useEffect } from "react";
import { conversationKey, isChannel, useIrc } from "../../../irc/IrcContext";
import {
  applyBancho,
  BANCHO_BOT,
  isMember,
  kickCommand,
  matchSlots,
  MATCH_STARTED,
  moveCommands,
  normalizeName,
  parseJoined,
  parseLeft,
  sameName,
  type Placement,
  type RoomMember,
  type RoomSlot,
  type RoomState,
} from "../../../lib/bancho";

// What the referee page does in a room by itself: puts bracket players in their slot and on
// their team's colour as they join, kicks anyone who joins without belonging there, and asks
// for !mp settings as each map starts, so every player's mods (picked under freemod) are
// read for the results. The referee page registers its room here instead of listening
// itself, so it keeps working after the referee switches to another match in the same tab.
// Every referee page on a match sees the same lines, in this browser or any other, so only
// one tab answers: each tab asks the server for the room, and the server picks one whose
// Bancho connection is in the room's channel, the match's referee before hosts
// (RoomRunners in handlers/irc.rs). When that tab leaves the channel or closes, the next in
// line takes over.

export interface RoomAutomationOptions {
  /** Bracket players, with the slot and colour each belongs in. */
  players: Placement[];
  /** Everyone allowed in the room: the match's players, and the hosts, referees and streamers. */
  members: RoomMember[];
  /** Bracket matches: moves joining players to their slot and colour. */
  autoPlace: boolean;
  /** Kicks anyone who joins without being one of `members`. */
  autoKick: boolean;
  /**
   * The room as the referee page last read it, so a join is placed knowing who holds which slot.
   * The automation keeps it up to date from BanchoBot's lines while the page is elsewhere.
   */
  room: RoomState;
}

interface Registration extends RoomAutomationOptions {
  /** "#mp_123" */
  channel: string;
  /**
   * Whether the server picked this tab to run the room. When it says this tab can't (the match
   * was concluded or given to another referee), the room stays registered but idle until the
   * page unregisters it or the connection is made again.
   */
  owned: boolean;
  /**
   * Who joined under a name no member has, by normalizeName, waiting for !mp settings to give
   * their osu! id: a player who renamed on osu! is told apart from a stranger by it.
   */
  unknown: Set<string>;
}

// This tab's rooms, by conversationKey of the channel.
const rooms = new Map<string, Registration>();

// Asks the server to run a room (or stops asking), while this tab is connected to Bancho.
let claim: ((channel: string, on: boolean) => void) | null = null;

/** Starts (or updates) running a room's channel. Stays on after the page closes. */
export function registerRoom(channel: string, options: RoomAutomationOptions) {
  const key = conversationKey(channel);
  const existing = rooms.get(key);
  if (existing) {
    Object.assign(existing, options);
    return;
  }
  rooms.set(key, { ...options, channel, owned: false, unknown: new Set() });
  claim?.(channel, true);
}

export function unregisterRoom(channel: string) {
  const key = conversationKey(channel);
  if (!rooms.has(key)) return;
  rooms.delete(key);
  claim?.(channel, false);
}

/** Listens to BanchoBot in the registered rooms. Mounted once, in AdminLayout. */
export function RoomAutomation() {
  const { onMessage, send, status, automate, onAutomation } = useIrc();

  // Every connection to the relay is new to the server, so the rooms are asked for again
  // each time this tab is online; while it isn't, it runs none.
  useEffect(() => {
    if (status !== "online") {
      for (const reg of rooms.values()) reg.owned = false;
      return;
    }
    claim = automate;
    for (const reg of rooms.values()) automate(reg.channel, true);
    return () => {
      claim = null;
    };
  }, [status, automate]);

  useEffect(
    () =>
      onAutomation((channel, state) => {
        const reg = rooms.get(conversationKey(channel));
        if (reg) reg.owned = state === "running";
      }),
    [onAutomation],
  );

  useEffect(() => {
    // Puts a player in their slot and colour. A slot another player holds is swapped, and
    // anyone else is moved out of the match's slots (see moveCommands); if Bancho refuses,
    // the referee page shows the player out of place and the referee moves them by hand.
    function place(channel: string, reg: Registration, slot: RoomSlot, player: Placement) {
      if (!reg.autoPlace) return;
      const mover = { ...slot, userId: player.osuId };
      const commands = moveCommands(reg.room, mover, player.slot, player.team, matchSlots(reg.players));
      if (commands.length === 0) return;
      for (const line of commands) send(channel, line);
      // Bancho doesn't say much about !mp team; the room is read again so the pages show it.
      send(channel, "!mp settings");
    }

    // Someone whose osu! id is known now: a player goes to their place, a stranger is kicked.
    function settle(channel: string, reg: Registration, slot: RoomSlot) {
      const player = reg.players.find((p) => p.osuId === slot.userId);
      if (player) return place(channel, reg, slot, player);
      if (reg.autoKick && !isMember(slot, reg.members)) send(channel, kickCommand(slot));
    }

    return onMessage((msg) => {
      if (msg.from !== BANCHO_BOT || !isChannel(msg.target)) return;
      const reg = rooms.get(conversationKey(msg.target));
      if (!reg) return;
      // Every tab follows the room, so the one that takes over knows it too.
      reg.room = applyBancho(reg.room, msg.text);
      if (!reg.owned) return;
      // The mods everyone plays the map with are settled now.
      if (msg.text === MATCH_STARTED) {
        send(msg.target, "!mp settings");
        return;
      }

      // !mp settings gave the ids of those who joined under an unknown name. (Its "Players: N"
      // empties the slots before listing them again, so nobody is dropped for being missing.)
      if (reg.unknown.size > 0) {
        for (const slot of reg.room.slots) {
          const name = normalizeName(slot.username);
          if (slot.userId === null || !reg.unknown.has(name)) continue;
          reg.unknown.delete(name);
          settle(msg.target, reg, slot);
        }
        // Whoever left before their id came needs nothing more.
        const left = parseLeft(msg.text);
        if (left) reg.unknown.delete(normalizeName(left));
      }

      const joined = parseJoined(msg.text);
      if (!joined) return;
      const slot = reg.room.slots.find((s) => sameName(s.username, joined.username));
      if (!slot) return;
      const player = reg.players.find((p) => sameName(p.username, joined.username));
      if (player) return place(msg.target, reg, slot, player);
      if (isMember(slot, reg.members)) return;
      if (!reg.autoKick && !reg.autoPlace) return;
      // A name no member has: a stranger, or someone who renamed on osu! since the site last
      // saw them. The join doesn't give an osu! id, so the room's settings are asked for, and
      // they're settled by id once it comes.
      reg.unknown.add(normalizeName(joined.username));
      send(msg.target, "!mp settings");
    });
  }, [onMessage, send]);
  return null;
}
