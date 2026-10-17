import { useEffect } from "react";
import { conversationKey, isChannel, useIrc } from "../../../irc/IrcContext";
import {
  applyBancho,
  BANCHO_BOT,
  matchSlots,
  MATCH_STARTED,
  moveCommands,
  parseJoined,
  sameName,
  type Placement,
  type RoomState,
} from "../../../lib/bancho";

// What the referee page does in a room by itself: puts bracket players in their slot and on
// their team's colour as they join, and asks for !mp settings as each map starts, so every
// player's mods (picked under freemod) are read for the results. The referee page
// registers its room here instead of listening itself, so it keeps working after the
// referee switches to another match in the same tab. Every tab shares the Bancho
// connection and sees the same lines, so a Web Lock per room picks the one tab that
// answers; when that tab closes, the next one waiting for the lock takes over.

export interface RoomAutomationOptions {
  /** Bracket players, with the slot and colour each belongs in. */
  players: Placement[];
  /** Bracket matches: moves joining players to their slot and colour. */
  autoPlace: boolean;
  /**
   * The room as the referee page last read it, so a join is placed knowing who holds which slot.
   * The automation keeps it up to date from BanchoBot's lines while the page is elsewhere.
   */
  room: RoomState;
}

interface Registration extends RoomAutomationOptions {
  /** Whether this tab holds the room's lock. */
  owned: boolean;
  release: () => void;
}

// This tab's rooms, by conversationKey of the channel.
const rooms = new Map<string, Registration>();

/** Starts (or updates) running a room's channel. Stays on after the page closes. */
export function registerRoom(channel: string, options: RoomAutomationOptions) {
  const key = conversationKey(channel);
  const existing = rooms.get(key);
  if (existing) {
    Object.assign(existing, options);
    return;
  }
  const reg: Registration = { ...options, owned: false, release: () => {} };
  rooms.set(key, reg);
  if (!("locks" in navigator)) {
    reg.owned = true;
    return;
  }
  const abort = new AbortController();
  let unlock = () => {};
  reg.release = () => {
    abort.abort();
    unlock();
  };
  navigator.locks
    .request(`room-automation:${key}`, { signal: abort.signal }, () => {
      reg.owned = true;
      return new Promise<void>((resolve) => (unlock = resolve));
    })
    .catch(() => {
      // released while still waiting for the lock
    });
}

export function unregisterRoom(channel: string) {
  const key = conversationKey(channel);
  rooms.get(key)?.release();
  rooms.delete(key);
}

/** Listens to BanchoBot in the registered rooms. Mounted once, in AdminLayout. */
export function RoomAutomation() {
  const { onMessage, send } = useIrc();
  useEffect(
    () =>
      onMessage((msg) => {
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
        const joined = reg.autoPlace && parseJoined(msg.text);
        if (!joined) return;
        const player = reg.players.find((p) => sameName(p.username, joined.username));
        const slot = reg.room.slots.find((s) => sameName(s.username, joined.username));
        if (!player || !slot) return;
        // A slot another player holds is swapped, and anyone else is moved out of the match's
        // slots (see moveCommands); if Bancho refuses, the referee page shows the player out of
        // place and the referee moves them by hand.
        const mover = { ...slot, userId: player.osuId };
        const commands = moveCommands(reg.room, mover, player.slot, player.team, matchSlots(reg.players));
        if (commands.length === 0) return;
        for (const line of commands) send(msg.target, line);
        // Bancho doesn't say much about !mp team; the room is read again so the pages show it.
        send(msg.target, "!mp settings");
      }),
    [onMessage, send],
  );
  return null;
}
