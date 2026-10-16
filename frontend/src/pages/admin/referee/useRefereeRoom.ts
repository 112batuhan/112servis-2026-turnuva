import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  addManualResult,
  forfeitMatch,
  removeManualResult,
  removeMapBan,
  removeMapPick,
  setBracketScore,
  setMapBan,
  setMapPick,
} from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { addMatchRoom, removeMatchRoom } from "../../../api/matches";
import { fetchRefereeRoom, fetchRoomChat } from "../../../api/referee";
import type { AdminTeamPlayer, PublicPoolMap, RefereeRoom, Team } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import {
  applyBancho,
  BANCHO_BOT,
  EMPTY_ROOM,
  ircName,
  modsCommand,
  parseCreated,
  playerRef,
  ROOM_PREFIX,
  sameName,
  TEAM_MODE,
  type MapResult,
  type RoomSlot,
  type RoomState,
  type ScoreMode,
  type SlotTeam,
} from "../../../lib/bancho";
import { matchCode } from "../../../lib/bracket";
import { readJson, writeJson } from "../../../lib/storage";
import { registerAutoTeams, unregisterAutoTeams } from "./autoTeams";
import { poolEntries, type PoolEntry } from "./pool";
import { mergeChat, toChatLine, type SavedChat } from "./roomChat";

export interface RosterPlayer {
  player: AdminTeamPlayer;
  team: Team<AdminTeamPlayer>;
  /** Bracket matches: the colour the player's team plays as (team 1 red, team 2 blue). */
  color: SlotTeam | null;
}

export interface RoomOptions {
  scoreMode: ScoreMode;
  /** Adds NoFail to every non-freemod map. */
  noFail: boolean;
  /** Bracket matches: moves players to their team's colour as they join. */
  autoTeams: boolean;
}

/** One of the match's rooms in the chat: its saved lines followed by the live ones. */
export interface RoomChatView {
  mpId: number;
  /** "#mp_<id>" */
  channel: string;
  url: string;
  lines: ChatLine[];
  /** Whether we're in its channel right now. */
  joined: boolean;
  /** The match's latest room, where it's being played. */
  current: boolean;
}

export interface TeamTotal {
  teamId: string;
  score: number;
}

/** A finished map of the pool, read from the room, with its place among all finished maps. */
export interface ResultRow {
  result: MapResult;
  /** Index among every finished map, which "Sayma" is keyed by. */
  index: number;
  entry: PoolEntry;
  /** Each team's summed score on it, best first. */
  totals: TeamTotal[];
  /** The team that won it, or null for a tie or a map without scores. */
  winnerId: string | null;
}

/** A bracket match's map wins, from the room and the referee's hand-awarded results. */
export interface Tally {
  rows: ResultRow[];
  /** Finished maps that aren't in the pool (warmups, mistakes): not shown, not counted. */
  outsidePool: number;
  team1: number;
  team2: number;
}

// How long BanchoBot gets to answer !mp make.
const CREATE_TIMEOUT_MS = 20_000;

const PLAY_MODES: Record<string, number> = { osu: 0, taiko: 1, fruits: 2, mania: 3 };

const DEFAULT_OPTIONS: RoomOptions = { scoreMode: 3, noFail: true, autoTeams: true };

// Each match's options, so switching to another match and back keeps them. For this
// tab only, until it's reloaded.
const matchOptions = new Map<string, RoomOptions>();

// The finished maps the referee doesn't count, per match, kept in the browser: the
// running score is saved from them, so a reload mustn't start counting them again.
const ignoredKey = (matchId: string) => `referee-ignored:${matchId}`;

// The tabs share one Bancho connection, and each takes BanchoBot's next "Created the
// tournament match" reply as its own, so only one !mp make is out at a time. A Web
// Lock covers every tab of this browser; the queue is for browsers without one.
let makeQueue: Promise<unknown> = Promise.resolve();
function oneAtATime<T>(task: () => Promise<T>): Promise<T> {
  if ("locks" in navigator) return navigator.locks.request("bancho-mp-make", task);
  const result = makeQueue.then(task, task);
  makeQueue = result.catch(() => {});
  return result;
}

/** "Lobi 3" or "W2". */
export function roomLabel(room: RefereeRoom): string {
  if (room.kind === "qualifier") return `Lobi ${room.lobby_number ?? "?"}`;
  return room.side && room.number !== null ? matchCode({ side: room.side, number: room.number }) : "Maç";
}

/** The osu! room name: "112S: (Team A) vs (Team B)" or "112S: Qualifiers Lobby 3". */
export function roomName(room: RefereeRoom): string {
  if (room.kind === "qualifier") return `${ROOM_PREFIX}: Qualifiers Lobby ${room.lobby_number ?? ""}`.trim();
  const name = (id: string | null) => room.teams.find((t) => t.id === id)?.name ?? "TBD";
  return `${ROOM_PREFIX}: (${name(room.team1_id)}) vs (${name(room.team2_id)})`;
}

function readLobby(lines: ChatLine[]): RoomState {
  return lines.filter((l) => l.from === BANCHO_BOT && !l.notice).reduce((r, l) => applyBancho(r, l.text), EMPTY_ROOM);
}

/** Each team's summed score on a map, best first. Players not on the roster don't count. */
export function teamTotals(result: MapResult, roster: RosterPlayer[]): TeamTotal[] {
  const totals = new Map<string, number>();
  for (const s of result.scores) {
    const r = roster.find((p) => sameName(p.player.username, s.username));
    if (r) totals.set(r.team.id, (totals.get(r.team.id) ?? 0) + s.score);
  }
  return [...totals].map(([teamId, score]) => ({ teamId, score })).sort((a, b) => b.score - a.score);
}

/** Owns the referee page's data, the room state read from the chat, and every command it sends. */
export function useRefereeRoom(matchId: string) {
  const user = useSignedInUser();
  const irc = useIrc();
  const [room, setRoom] = useState<RefereeRoom | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [options, setOptions] = useState<RoomOptions>(() => matchOptions.get(matchId) ?? DEFAULT_OPTIONS);
  useEffect(() => {
    matchOptions.set(matchId, options);
  }, [matchId, options]);
  // Finished maps the referee doesn't count (warmups, replays), by index.
  const [ignored, setIgnored] = useState<Set<number>>(() => new Set(readJson<number[]>(ignoredKey(matchId), [])));
  useEffect(() => {
    writeJson(ignoredKey(matchId), [...ignored]);
  }, [matchId, ignored]);
  // Each room's saved chat, by room id.
  const [saved, setSaved] = useState<Map<number, SavedChat>>(new Map());
  // The rooms the saved chat was last loaded for, so the score isn't read from a half-loaded chat.
  const [savedFor, setSavedFor] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setRoom(await fetchRefereeRoom(matchId));
      setLoadError(null);
    } catch (e) {
      setLoadError(errorMessage(e));
    }
  }, [matchId]);

  useEffect(() => {
    reload();
  }, [reload]);

  const concluded = room !== null && room.concluded_at !== null;
  const rooms = useMemo(() => room?.rooms ?? [], [room]);
  const current = rooms.length > 0 ? rooms[rooms.length - 1] : null;
  const channel = current ? `#mp_${current.mp_id}` : null;
  const conversation = channel ? irc.conversations[conversationKey(channel)] : undefined;
  const online = irc.status === "online";
  const inChannel = online && conversation?.joined === true;
  const canRun = room !== null && (hasRole(user, "host") || room.referee?.id === user.id);

  // Loads every room's saved chat whenever the match's rooms change. Live lines up to
  // this point are in it, so only later ones are added on top (see mergeChat).
  const conversations = useRef(irc.conversations);
  conversations.current = irc.conversations;
  const roomsKey = rooms.map((r) => r.mp_id).join(",");
  useEffect(() => {
    setSaved(new Map());
    setSavedFor(null);
    if (!roomsKey) {
      setSavedFor("");
      return;
    }
    const markers = new Map(
      roomsKey.split(",").map((id) => {
        const lines = conversations.current[conversationKey(`#mp_${id}`)]?.lines ?? [];
        return [Number(id), lines.length > 0 ? lines[lines.length - 1].id : 0];
      }),
    );
    let live = true;
    fetchRoomChat(matchId)
      .then((chats) => {
        if (!live) return;
        const fetchedAt = Date.now();
        setSaved(
          new Map(
            chats.map((c) => [
              c.mp_id,
              { lines: c.messages.map(toChatLine), marker: markers.get(c.mp_id) ?? 0, fetchedAt },
            ]),
          ),
        );
        setSavedFor(roomsKey);
      })
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [matchId, roomsKey]);

  const roomChats = useMemo<RoomChatView[]>(
    () =>
      rooms.map((r, i) => {
        const channel = `#mp_${r.mp_id}`;
        const conversation = irc.conversations[conversationKey(channel)];
        return {
          mpId: r.mp_id,
          channel,
          url: r.url,
          lines: mergeChat(saved.get(r.mp_id), conversation?.lines ?? []),
          joined: online && conversation?.joined === true,
          current: i === rooms.length - 1,
        };
      }),
    [rooms, saved, irc.conversations, online],
  );

  // The latest room as BanchoBot has described it, with the finished maps of every
  // room the match has been played in.
  const lobby = useMemo<RoomState>(() => {
    const states = roomChats.map((c) => readLobby(c.lines));
    const latest = states.length > 0 ? states[states.length - 1] : EMPTY_ROOM;
    return { ...latest, results: states.flatMap((st) => st.results) };
  }, [roomChats]);

  const roster = useMemo<RosterPlayer[]>(() => {
    if (!room) return [];
    const color = (team: Team<AdminTeamPlayer>): SlotTeam | null =>
      room.kind !== "bracket" ? null : team.id === room.team1_id ? "red" : team.id === room.team2_id ? "blue" : null;
    return room.teams.flatMap((team) => team.players.map((player) => ({ player, team, color: color(team) })));
  }, [room]);

  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find((r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username)),
    [roster],
  );

  // The match's map wins: each finished pool map goes to the team with the higher total,
  // unless the referee left it out, plus the maps they awarded by hand.
  const tally = useMemo<Tally | null>(() => {
    if (!room || room.kind !== "bracket") return null;
    const entries = poolEntries(room);
    const rows: ResultRow[] = lobby.results.flatMap((result, index) => {
      const entry = entries.find((e) => e.map.beatmap_id === result.beatmapId);
      if (!entry) return [];
      const totals = teamTotals(result, roster);
      const [best, second] = totals;
      const winnerId = best && !(second && second.score === best.score) ? best.teamId : null;
      return [{ result, index, entry, totals, winnerId }];
    });
    const wins = (teamId: string | null) =>
      teamId === null
        ? 0
        : rows.filter((r) => !ignored.has(r.index) && r.winnerId === teamId).length +
          room.manual_results.filter((m) => m.team_id === teamId).length;
    return {
      rows,
      outsidePool: lobby.results.length - rows.length,
      team1: wins(room.team1_id),
      team2: wins(room.team2_id),
    };
  }, [room, lobby.results, roster, ignored]);

  // The running score is saved as it changes, so the site shows it live: when a map
  // finishes, a result is awarded by hand or a map is left out. Not on the first
  // reading after the page loads, which would overwrite a score entered elsewhere
  // (the referee can still save that one by hand).
  const liveScore =
    tally !== null &&
    canRun &&
    !concluded &&
    room?.team1_id != null &&
    room?.team2_id != null &&
    savedFor === roomsKey;
  const lastTally = useRef<string | null>(null);
  const [scoreSaving, setScoreSaving] = useState(false);
  useEffect(() => {
    if (!liveScore || !room || !tally) {
      lastTally.current = null;
      return;
    }
    const key = `${tally.team1}:${tally.team2}`;
    const previous = lastTally.current;
    lastTally.current = key;
    if (previous === null || previous === key) return;
    if (room.team1_score === tally.team1 && room.team2_score === tally.team2) return;
    const { team1, team2 } = tally;
    setScoreSaving(true);
    setBracketScore(room.id, team1, team2)
      .then(() => setRoom((r) => r && { ...r, team1_score: team1, team2_score: team2 }))
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setScoreSaving(false));
  }, [liveScore, room, tally]);

  const say = useCallback(
    (text: string, target: string | null = channel) => {
      if (target) irc.send(target, text);
    },
    [channel, irc],
  );

  // Joins the room's channel once per connection, so a reconnect gets back in. After
  // that it's up to the referee: a closed room's channel stays closed.
  const { join, status } = irc;
  const autoJoined = useRef(new Set<string>());
  useEffect(() => {
    if (status !== "online") {
      autoJoined.current.clear();
      return;
    }
    if (!channel || concluded || autoJoined.current.has(channel)) return;
    autoJoined.current.add(channel);
    if (!conversation?.joined) join(channel);
  }, [status, channel, concluded, conversation?.joined, join]);

  // Puts bracket players on their team's colour as they join, also while the referee
  // is on another match (see autoTeams.ts). Replaced rooms stop.
  const autoTeams = room?.kind === "bracket" && canRun && !concluded && options.autoTeams;
  useEffect(() => {
    for (const r of rooms.slice(0, -1)) unregisterAutoTeams(`#mp_${r.mp_id}`);
    if (!channel) return;
    if (!autoTeams) return unregisterAutoTeams(channel);
    registerAutoTeams(
      channel,
      roster.flatMap((r) =>
        r.color ? [{ osuId: r.player.osu_id, username: r.player.username, color: r.color }] : [],
      ),
    );
  }, [rooms, channel, autoTeams, roster]);
  const { onMessage } = irc;

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  // Team mode, score mode and size; locks the slots, then invites everyone.
  function setUp(target: string) {
    if (!room) return;
    const players = roster.map((r) => r.player);
    const mode = room.kind === "bracket" ? TEAM_MODE.teamVs : TEAM_MODE.headToHead;
    const size = Math.min(16, Math.max(2, players.length));
    say(`!mp set ${mode} ${options.scoreMode} ${size}`, target);
    say("!mp lock", target);
    for (const p of players) say(`!mp invite ${playerRef(p.osu_id)}`, target);
    say("!mp settings", target);
  }

  // Asks BanchoBot for a tournament room and waits for the room id in its reply.
  function makeRoom(name: string): Promise<number> {
    return oneAtATime(
      () =>
        new Promise((resolve, reject) => {
          const off = onMessage((msg) => {
            if (msg.from !== BANCHO_BOT || isChannel(msg.target)) return;
            const created = parseCreated(msg.text);
            if (created) {
              done();
              resolve(created.mpId);
            } else if (/cannot create/i.test(msg.text)) {
              done();
              reject(new Error(msg.text));
            }
          });
          const timer = window.setTimeout(() => {
            done();
            reject(new Error("BanchoBot cevap vermedi. Sohbette ne dediğine bak."));
          }, CREATE_TIMEOUT_MS);
          function done() {
            off();
            window.clearTimeout(timer);
          }
          irc.send(BANCHO_BOT, `!mp make ${name}`);
        }),
    );
  }

  const actions = {
    reload,
    setOptions,
    say,

    /** !mp make, adds the room to the match, then sets it up. Also replaces a broken room. */
    createRoom: () =>
      run(async () => {
        if (!room) return;
        const mpId = await makeRoom(roomName(room));
        setUp(`#mp_${mpId}`);
        await addMatchRoom(room.id, String(mpId));
        await reload();
      }),

    /** Adds a room made some other way. */
    addRoom: (link: string) =>
      run(async () => {
        if (!room) return;
        await addMatchRoom(room.id, link);
        await reload();
      }),

    removeRoom: (mpId: number) =>
      run(async () => {
        if (!room) return;
        await removeMatchRoom(room.id, mpId);
        await reload();
      }),

    setUpRoom: () => channel && setUp(channel),
    invite: (osuId: number) => say(`!mp invite ${playerRef(osuId)}`),
    inviteAll: () => roster.forEach((r) => say(`!mp invite ${playerRef(r.player.osu_id)}`)),
    moveToTeam: (slot: Pick<RoomSlot, "userId" | "username">, team: SlotTeam) =>
      say(`!mp team ${slot.userId ? playerRef(slot.userId) : ircName(slot.username)} ${team}`),

    /** Puts everyone in the room on their team's colour. */
    fixTeams: () => {
      for (const slot of lobby.slots) {
        const r = findRoster(slot);
        if (r?.color && slot.team !== r.color) say(`!mp team ${playerRef(r.player.osu_id)} ${r.color}`);
      }
    },

    /** Sets a map and its mods in the room, without recording who picked it. */
    pickMap: (map: PublicPoolMap, freemod: boolean) => {
      say(`!mp map ${map.beatmap_id} ${PLAY_MODES[map.mode] ?? 0}`);
      say(modsCommand(map.mods, freemod, options.noFail));
    },

    /** Records a team's pick, then sets the map in the room when we're in it. */
    pick: (map: PublicPoolMap, teamId: string, freemod: boolean) =>
      run(async () => {
        if (!room) return;
        await setMapPick(room.id, map.id, teamId);
        if (inChannel) {
          say(`!mp map ${map.beatmap_id} ${PLAY_MODES[map.mode] ?? 0}`);
          say(modsCommand(map.mods, freemod, options.noFail));
        }
        await reload();
      }),

    unpick: (mapId: string) =>
      run(async () => {
        if (!room) return;
        await removeMapPick(room.id, mapId);
        await reload();
      }),

    settings: () => say("!mp settings"),
    lock: (locked: boolean) => say(locked ? "!mp lock" : "!mp unlock"),
    timer: (seconds: number) => say(`!mp timer ${seconds}`),
    abortTimer: () => say("!mp aborttimer"),
    start: (seconds: number) => say(seconds > 0 ? `!mp start ${seconds}` : "!mp start"),
    abort: () => say("!mp abort"),
    close: () => say("!mp close"),
    rejoin: () => channel && join(channel),

    toggleIgnored: (index: number) =>
      setIgnored((prev) => {
        const next = new Set(prev);
        if (!next.delete(index)) next.add(index);
        return next;
      }),

    ban: (mapId: string, teamId: string) =>
      run(async () => {
        if (!room) return;
        await setMapBan(room.id, mapId, teamId);
        await reload();
      }),

    unban: (mapId: string) =>
      run(async () => {
        if (!room) return;
        await removeMapBan(room.id, mapId);
        await reload();
      }),

    saveScore: (team1: number, team2: number) =>
      run(async () => {
        if (!room) return;
        await setBracketScore(room.id, team1, team2);
        await reload();
      }),

    /** Awards a map (or, without one, a point) to a team by hand; it counts towards the score. */
    addManualResult: (teamId: string, mapId: string | null, note: string) =>
      run(async () => {
        if (!room) return;
        await addManualResult(room.id, teamId, mapId, note);
        await reload();
      }),

    removeManualResult: (resultId: string) =>
      run(async () => {
        if (!room) return;
        await removeManualResult(room.id, resultId);
        await reload();
      }),

    /** Concludes the match by forfeit: `slot` is the team that forfeited; the other one moves on. */
    forfeit: (slot: 1 | 2) =>
      run(async () => {
        if (!room) return;
        await forfeitMatch(room.id, slot);
        await reload();
      }),
  };

  return {
    room,
    roomChats,
    loadError,
    error,
    busy,
    canRun,
    channel,
    online,
    inChannel,
    lobby,
    roster,
    findRoster,
    options,
    ignored,
    tally,
    liveScore,
    scoreSaving,
    actions,
  };
}

export type RefereeRoomState = ReturnType<typeof useRefereeRoom>;
