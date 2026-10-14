import { useEffect } from "react";
import { conversationKey, isChannel, useIrc } from "../../../irc/IrcContext";
import { BANCHO_BOT, parseJoined, playerRef, sameName, type SlotTeam } from "../../../lib/bancho";

// Moves bracket players to their team's colour as they join a room. The referee page
// registers its room here instead of listening itself, so it keeps working after the
// referee switches to another match in the same tab. Every tab shares the Bancho
// connection and sees the same lines, so a Web Lock per room picks the one tab that
// answers; when that tab closes, the next one waiting for the lock takes over.

export interface AutoTeamsPlayer {
  osuId: number;
  username: string;
  color: SlotTeam;
}

interface Registration {
  players: AutoTeamsPlayer[];
  /** Whether this tab holds the room's lock. */
  owned: boolean;
  release: () => void;
}

// This tab's rooms, by conversationKey of the channel.
const rooms = new Map<string, Registration>();

/** Starts (or updates) moving players in a room's channel. Stays on after the page closes. */
export function registerAutoTeams(channel: string, players: AutoTeamsPlayer[]) {
  const key = conversationKey(channel);
  const existing = rooms.get(key);
  if (existing) {
    existing.players = players;
    return;
  }
  const reg: Registration = { players, owned: false, release: () => {} };
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
    .request(`auto-teams:${key}`, { signal: abort.signal }, () => {
      reg.owned = true;
      return new Promise<void>((resolve) => (unlock = resolve));
    })
    .catch(() => {
      // released while still waiting for the lock
    });
}

export function unregisterAutoTeams(channel: string) {
  const key = conversationKey(channel);
  rooms.get(key)?.release();
  rooms.delete(key);
}

/** Listens for players joining the registered rooms. Mounted once, in AdminLayout. */
export function AutoTeams() {
  const { onMessage, send } = useIrc();
  useEffect(
    () =>
      onMessage((msg) => {
        if (msg.from !== BANCHO_BOT || !isChannel(msg.target)) return;
        const reg = rooms.get(conversationKey(msg.target));
        const joined = reg?.owned && parseJoined(msg.text);
        if (!reg || !joined) return;
        const player = reg.players.find((p) => sameName(p.username, joined.username));
        if (player && joined.team !== player.color) {
          send(msg.target, `!mp team ${playerRef(player.osuId)} ${player.color}`);
        }
      }),
    [onMessage, send],
  );
  return null;
}
