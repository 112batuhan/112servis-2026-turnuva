import { useEffect, useRef } from "react";
import { useAuth } from "../../../auth/AuthContext";
import { conversationKey, isChannel, useIrc } from "../../../irc/IrcContext";
import {
  addRefCommand,
  applyBancho,
  BANCHO_BOT,
  isRoomRef,
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
// their team's colour as they join, kicks anyone who joins without belonging there, asks
// for !mp settings as each map starts, so every player's mods (picked under freemod) are
// read for the results, puts the tiebreaker on once the score reaches it, and makes every
// other referee who joins the room a referee of it (!mp addref), so they can join its
// channel and their tournament client shows its chat. The referee page registers its room here instead of listening
// itself, so it keeps working after the referee switches to another match in the same tab.
// Every referee page on a match sees the same lines, in this browser or any other, so only
// one tab answers: each tab asks the server for the room, and the server picks one whose
// Bancho connection is in the room's channel, the match's referee before hosts
// (RoomRunners in handlers/irc.rs). When that tab leaves the channel or closes, the next in
// line takes over.

/** The tiebreaker the room should go to, while the score is tied one map short of the best of. */
export interface TiebreakerCue {
  /** The score it's for ("4-4"): it's sent once per score. */
  key: string;
  beatmapId: number;
  /** The score line, !mp map and !mp mods. */
  commands: string[];
}

export interface RoomAutomationOptions {
  /** Bracket players, with the slot and colour each belongs in. */
  players: Placement[];
  /** Everyone allowed in the room: the match's players, and the hosts, referees and streamers. */
  members: RoomMember[];
  /** The holders of Referee: whoever of them joins the room is made a referee of it. */
  referees: RoomMember[];
  /** Bracket matches: moves joining players to their slot and colour. */
  autoPlace: boolean;
  /** Kicks anyone who joins without being one of `members`. */
  autoKick: boolean;
  /**
   * The room as the referee page last read it, so a join is placed knowing who holds which slot.
   * The automation keeps it up to date from BanchoBot's lines while the page is elsewhere.
   */
  room: RoomState;
  /** Set while the tiebreaker is due. */
  tiebreaker: TiebreakerCue | null;
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
  /** TiebreakerCue.key of the one already handled. */
  tiebreakerDone: string | null;
  /** The referees (by osu! id) already sent !mp addref, so a rejoin doesn't send it again. */
  refsAsked: Set<number>;
}

// This tab's rooms, by conversationKey of the channel.
const rooms = new Map<string, Registration>();

// Asks the server to run a room (or stops asking), while this tab is connected to Bancho.
let claim: ((channel: string, on: boolean) => void) | null = null;

// Puts the tiebreaker on when it's due, from the tab that runs the room.
let cueTiebreaker: ((reg: Registration) => void) | null = null;

/** Starts (or updates) running a room's channel. Stays on after the page closes. */
export function registerRoom(channel: string, options: RoomAutomationOptions) {
  const key = conversationKey(channel);
  const existing = rooms.get(key);
  if (existing) {
    Object.assign(existing, options);
    cueTiebreaker?.(existing);
    return;
  }
  rooms.set(key, { ...options, channel, owned: false, unknown: new Set(), tiebreakerDone: null, refsAsked: new Set() });
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
  // The signed-in referee: already a referee of the rooms they run, so never added.
  const { user } = useAuth();
  const self = useRef<number | null>(null);
  self.current = user?.osu_id ?? null;

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
        if (!reg) return;
        reg.owned = state === "running";
        cueTiebreaker?.(reg);
      }),
    [onAutomation],
  );

  // The tiebreaker goes on between maps, once per score, and only while the room still has
  // the map that ended (or none): a map the referee put on since is theirs to change.
  useEffect(() => {
    cueTiebreaker = (reg) => {
      const cue = reg.tiebreaker;
      if (!reg.owned || !cue || reg.tiebreakerDone === cue.key || reg.room.playing) return;
      const last = reg.room.results[reg.room.results.length - 1];
      reg.tiebreakerDone = cue.key;
      if (reg.room.beatmapId === cue.beatmapId) return;
      if (reg.room.beatmapId !== null && reg.room.beatmapId !== last?.beatmapId) return;
      for (const line of cue.commands) send(reg.channel, line);
    };
    return () => {
      cueTiebreaker = null;
    };
  }, [send]);

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

    // A referee who joined the room is made one of its referees, once; by the name Bancho gave
    // the join, which is their current one. Whether they were someone the room's referees has.
    function addRef(channel: string, reg: Registration, slot: RoomSlot): boolean {
      const ref = reg.referees.find((r) =>
        slot.userId !== null ? r.osuId === slot.userId : sameName(r.username, slot.username),
      );
      if (!ref) return false;
      if (ref.osuId === self.current || reg.refsAsked.has(ref.osuId) || isRoomRef(reg.room, slot.username)) return true;
      reg.refsAsked.add(ref.osuId);
      send(channel, addRefCommand(slot.username));
      return true;
    }

    // Someone whose osu! id is known now: a player goes to their place, a referee is made one
    // of the room's, a stranger is kicked.
    function settle(channel: string, reg: Registration, slot: RoomSlot) {
      const player = reg.players.find((p) => p.osuId === slot.userId);
      if (player) return place(channel, reg, slot, player);
      if (addRef(channel, reg, slot)) return;
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
      if (addRef(msg.target, reg, slot)) return;
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
