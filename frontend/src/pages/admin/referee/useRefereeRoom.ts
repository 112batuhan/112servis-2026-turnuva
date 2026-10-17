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
import { fetchOsuGames, fetchRefereeRoom, fetchRoomChat } from "../../../api/referee";
import type {
  AdminTeamPlayer,
  MapPlayerScore,
  MapResultRow,
  OsuRoomGames,
  Player,
  PublicMatch,
  PublicPoolMap,
  RefereeRoom,
  Team,
} from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import {
  applyBancho,
  BANCHO_BOT,
  EMPTY_ROOM,
  isFreemod,
  matchSlots,
  MATCH_STARTED,
  modAcronyms,
  modsCommand,
  moveCommands,
  parseCreated,
  placeCommands,
  playerRef,
  ROOM_PREFIX,
  sameName,
  SIDE_COLOR,
  SIDE_SLOTS,
  splitMods,
  TEAM_MODE,
  type MapResult,
  type MapScore,
  type Placement,
  type RoomSlot,
  type RoomState,
  type ScoreMode,
  type SlotTeam,
} from "../../../lib/bancho";
import { matchCode } from "../../../lib/bracket";
import { readJson, writeJson } from "../../../lib/storage";
import { registerRoom, unregisterRoom } from "./roomAutomation";
import {
  entryOf,
  matchTeams,
  nextPicker,
  poolEntries,
  scoreMessage,
  type MatchData,
  type MatchTeam,
  type PoolEntry,
} from "./pool";
import { mergeChat, toChatLine, type SavedChat } from "./roomChat";

export interface RosterPlayer<P extends Player = Player> {
  player: P;
  team: Team<P>;
  /** Bracket matches: the colour the player's team plays as (team 1 blue, team 2 red). */
  color: SlotTeam | null;
  /**
   * Bracket matches: the room slot the player belongs in. Team 1's players take 1 and 2, team 2's
   * 3 and 4, in the order the team lists them; the pages list them in that order too.
   */
  home: number | null;
}

export interface RoomOptions {
  scoreMode: ScoreMode;
  /** Adds NoFail to every non-freemod map. */
  noFail: boolean;
  /** Bracket matches: moves players to their slot and their team's colour as they join. */
  autoPlace: boolean;
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

/**
 * Where the finished maps are read from: the rooms' chat, osu!'s record of the rooms, or, once
 * a bracket match is concluded, the map results saved with its score.
 */
export type ResultSource = "chat" | "osu" | "saved";

/** A finished map of one of the match's rooms. */
export interface FinishedMap {
  result: MapResult;
  /** "<source>:<room>:<n>", the nth finished map of the room; "Sayma" is keyed by it. */
  key: string;
}

/** A finished map of the pool, read from the room. */
export interface ResultRow {
  result: MapResult;
  /** FinishedMap.key */
  key: string;
  entry: PoolEntry;
  /** Each team's summed score on it, best first. */
  totals: TeamTotal[];
  /** The team that won it, or null for a tie or a map without scores. */
  winnerId: string | null;
}

/** A bracket match's map wins, from the room and the referee's hand-awarded results. */
export interface Tally {
  rows: ResultRow[];
  /** The counted rows as the site saves them with the score. */
  maps: MapResultRow[];
  /** Finished maps that aren't in the pool (warmups, mistakes): not shown, not counted. */
  outsidePool: number;
  team1: number;
  team2: number;
}

// How long BanchoBot gets to answer !mp make.
const CREATE_TIMEOUT_MS = 20_000;

const PLAY_MODES: Record<string, number> = { osu: 0, taiko: 1, fruits: 2, mania: 3 };

const DEFAULT_OPTIONS: RoomOptions = { scoreMode: 3, noFail: true, autoPlace: true };

// Each match's options, so switching to another match and back keeps them. For this
// tab only, until it's reloaded.
const matchOptions = new Map<string, RoomOptions>();

// The finished maps the referee doesn't count, per match, kept in the browser: the
// running score is saved from them, so a reload mustn't start counting them again.
const ignoredKey = (matchId: string) => `referee-ignored-maps:${matchId}`;

// The rooms as osu! recorded them, once the referee synced the results from there:
// kept in the browser, so a reload keeps counting from them rather than the chat.
interface OsuSync {
  rooms: OsuRoomGames[];
  /** When they were fetched, ISO. */
  at: string;
}
const osuKey = (matchId: string) => `referee-osu-games:${matchId}`;

// How long after the chat sees a map finish the osu! record is read again, so osu! has it too.
const OSU_REFETCH_MS = 5_000;

/**
 * osu!'s record of a room as finished maps; games nobody finished (aborted ones) are left out.
 * osu! doesn't say whether freemod was on, so a map counts as freemod when a player had mods the
 * room didn't give everyone (NoFail aside).
 */
function osuResults(games: NonNullable<OsuRoomGames["games"]>): MapResult[] {
  return games
    .filter((g) => g.scores.length > 0)
    .map((g) => {
      const roomMods = g.mods.join("");
      const room = new Set(splitMods(roomMods));
      const scores = g.scores.map((sc) => {
        // The room's mods first, as the chat lists them, then the player's own.
        const own = splitMods(sc.mods.join("")).filter((m) => !room.has(m));
        return {
          username: "",
          userId: sc.osu_id,
          score: sc.score,
          passed: sc.passed,
          mods: [...room, ...own].join(""),
        };
      });
      const freemod = scores.some((sc) => splitMods(sc.mods).some((m) => !room.has(m) && m !== "NF"));
      return { beatmapId: g.beatmap_id, beatmapName: null, roomMods, freemod, scores, teams: {} };
    });
}

/** What a saved score and its map results are compared by. */
function scoreKey(team1: number | null, team2: number | null, maps: MapResultRow[]): string {
  return JSON.stringify([
    team1,
    team2,
    maps.map((m) => [
      m.map_id,
      m.team1_score,
      m.team2_score,
      [...m.players]
        .sort((a, b) => b.score - a.score || a.user_id.localeCompare(b.user_id))
        .map((p) => [p.user_id, p.score, p.mods, p.passed]),
    ]),
  ]);
}

/** Each roster player's score on a map; players who aren't in the match aren't kept. */
function playerScores(result: MapResult, roster: RosterPlayer[]): MapPlayerScore[] {
  return result.scores.flatMap((s) => {
    const r = scorer(s, roster);
    return r ? [{ user_id: r.player.id, score: s.score, mods: s.mods, passed: s.passed }] : [];
  });
}

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
export function roomLabel(room: Pick<RefereeRoom | PublicMatch, "kind" | "lobby_number" | "side" | "number">): string {
  if (room.kind === "qualifier") return `Lobi ${room.lobby_number ?? "?"}`;
  return room.side && room.number !== null ? matchCode({ side: room.side, number: room.number }) : "Maç";
}

/** The osu! room name: "112S: (Team A) vs (Team B)" or "112S: Qualifiers Lobby 3". */
export function roomName(room: RefereeRoom): string {
  if (room.kind === "qualifier") return `${ROOM_PREFIX}: Qualifiers Lobby ${room.lobby_number ?? ""}`.trim();
  const name = (id: string | null) => room.teams.find((t) => t.id === id)?.name ?? "TBD";
  return `${ROOM_PREFIX}: (${name(room.team1_id)}) vs (${name(room.team2_id)})`;
}

/** A room as BanchoBot's lines in its chat describe it. */
export function readLobby(lines: Pick<ChatLine, "from" | "text" | "notice">[]): RoomState {
  return lines.filter((l) => l.from === BANCHO_BOT && !l.notice).reduce((r, l) => applyBancho(r, l.text), EMPTY_ROOM);
}

/** The roster player a score is by: by osu! id from osu!'s record, by name from the chat. */
export function scorer(score: MapScore, roster: RosterPlayer[]): RosterPlayer | undefined {
  return roster.find((p) =>
    score.userId !== undefined ? p.player.osu_id === score.userId : sameName(p.player.username, score.username),
  );
}

/** Each team's summed score on a map, best first. Players not on the roster don't count. */
export function teamTotals(result: MapResult, roster: RosterPlayer[]): TeamTotal[] {
  const totals = new Map<string, number>();
  for (const s of result.scores) {
    const r = scorer(s, roster);
    if (r) totals.set(r.team.id, (totals.get(r.team.id) ?? 0) + s.score);
  }
  return [...totals].map(([teamId, score]) => ({ teamId, score })).sort((a, b) => b.score - a.score);
}

/**
 * The map pool entry a room has on, given BanchoBot's description of it; picks tell apart a
 * beatmap the pool has twice.
 */
export function currentPoolEntry(room: MatchData, lobby: RoomState): PoolEntry | undefined {
  if (lobby.beatmapId === null) return undefined;
  return entryOf(
    poolEntries(room),
    { beatmapId: lobby.beatmapId, roomMods: modAcronyms(lobby.mods ?? ""), freemod: isFreemod(lobby.mods) },
    new Set(room.picks.map((p) => p.map_id)),
  );
}

/** When the map being played started: the room's last "The match has started!", so a reload keeps the clock. */
export function mapStartTime(lobby: RoomState, lines: Pick<ChatLine, "from" | "text" | "notice" | "at">[] | null): number | null {
  if (!lobby.playing || !lines) return null;
  for (let i = lines.length - 1; i >= 0; i--) {
    const l = lines[i];
    if (l.from === BANCHO_BOT && !l.notice && l.text === MATCH_STARTED) return l.at;
  }
  return null;
}

/** The match's players with their teams and, in a bracket match, the colour and slot each plays in. */
export function rosterOf<P extends Player>(
  room: Pick<MatchData, "kind" | "team1_id" | "team2_id"> & { teams: Team<P>[] },
): RosterPlayer<P>[] {
  const side = (team: Team<P>): 1 | 2 | null =>
    room.kind !== "bracket" ? null : team.id === room.team1_id ? 1 : team.id === room.team2_id ? 2 : null;
  return room.teams.flatMap((team) => {
    const s = side(team);
    return team.players.map((player, i) => ({
      player,
      team,
      color: s ? SIDE_COLOR[s] : null,
      home: s ? (SIDE_SLOTS[s][i] ?? null) : null,
    }));
  });
}

/** A match's saved map results as the results list shows them, with its map wins. */
export function savedTally(room: MatchData, entries: PoolEntry[], roster: RosterPlayer[]): Tally {
  const rows: ResultRow[] = room.map_results.flatMap((saved, i) => {
    const entry = entries.find((e) => e.map.id === saved.map_id);
    if (!entry) return [];
    const scores: MapScore[] = saved.players.map((p) => {
      const r = roster.find((r) => r.player.id === p.user_id);
      return { username: r?.player.username ?? "?", userId: r?.player.osu_id, score: p.score, passed: p.passed, mods: p.mods };
    });
    const totals = [
      { teamId: room.team1_id ?? "", score: saved.team1_score },
      { teamId: room.team2_id ?? "", score: saved.team2_score },
    ].sort((a, b) => b.score - a.score);
    const tied = saved.team1_score === saved.team2_score;
    return [
      {
        result: { beatmapId: entry.map.beatmap_id, beatmapName: null, roomMods: "", freemod: entry.freemod, scores, teams: {} },
        key: `saved:${i}`,
        entry,
        totals,
        winnerId: tied ? null : totals[0].teamId,
      },
    ];
  });
  const wins = (teamId: string | null) =>
    rows.filter((r) => teamId !== null && r.winnerId === teamId).length +
    room.manual_results.filter((m) => m.team_id === teamId).length;
  return { rows, maps: room.map_results, outsidePool: 0, team1: wins(room.team1_id), team2: wins(room.team2_id) };
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
  const [ignored, setIgnored] = useState<Set<string>>(() => new Set(readJson<string[]>(ignoredKey(matchId), [])));
  useEffect(() => {
    writeJson(ignoredKey(matchId), [...ignored]);
  }, [matchId, ignored]);
  // Set once the referee syncs the results from osu!; until then they're read from the chat.
  const [osuSync, setOsuSync] = useState<OsuSync | null>(() => readJson<OsuSync | null>(osuKey(matchId), null));
  useEffect(() => {
    writeJson(osuKey(matchId), osuSync);
  }, [matchId, osuSync]);
  const [syncing, setSyncing] = useState(false);
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
  const roomStates = useMemo(() => roomChats.map((c) => ({ mpId: c.mpId, state: readLobby(c.lines) })), [roomChats]);
  const lobby = useMemo<RoomState>(() => {
    const latest = roomStates.length > 0 ? roomStates[roomStates.length - 1].state : EMPTY_ROOM;
    return { ...latest, results: roomStates.flatMap((r) => r.state.results) };
  }, [roomStates]);

  // The pool map the room has on, and when it started if it's being played.
  const currentEntry = useMemo(() => (room ? currentPoolEntry(room, lobby) : undefined), [room, lobby]);
  const latestLines = roomChats.length > 0 ? roomChats[roomChats.length - 1].lines : null;
  const mapStartedAt = useMemo(() => mapStartTime(lobby, latestLines), [lobby, latestLines]);

  // Every room's finished maps, oldest room first: from osu!'s record of the room once
  // the referee synced it (unless osu! had none), otherwise from its chat.
  const readSource: ResultSource = osuSync ? "osu" : "chat";
  const finished = useMemo<FinishedMap[]>(
    () =>
      roomStates.flatMap(({ mpId, state }) => {
        const games = osuSync?.rooms.find((r) => r.mp_id === mpId)?.games;
        const [from, results] = games ? ["osu", osuResults(games)] : ["chat", state.results];
        return results.map((result, n) => ({ result, key: `${from}:${mpId}:${n}` }));
      }),
    [roomStates, osuSync],
  );

  const roster = useMemo<RosterPlayer<AdminTeamPlayer>[]>(() => (room ? rosterOf(room) : []), [room]);

  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find((r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username)),
    [roster],
  );

  // A concluded match's result stands: its map results as they were saved with the score
  // (the record the Discord results post is made from), not read from the chat again.
  const fromSaved = concluded && room?.kind === "bracket" && room.map_results.length > 0;
  const source: ResultSource = fromSaved ? "saved" : readSource;

  // The match's map wins: each finished pool map goes to the team with the higher total,
  // unless the referee left it out, plus the maps they awarded by hand.
  const tally = useMemo<Tally | null>(() => {
    if (!room || room.kind !== "bracket") return null;
    const entries = poolEntries(room);
    if (fromSaved) return savedTally(room, entries, roster);
    const picked = new Set(room.picks.map((p) => p.map_id));
    const rows: ResultRow[] = finished.flatMap(({ result, key }) => {
      const entry = entryOf(entries, result, picked);
      if (!entry) return [];
      const totals = teamTotals(result, roster);
      const [best, second] = totals;
      const winnerId = best && !(second && second.score === best.score) ? best.teamId : null;
      return [{ result, key, entry, totals, winnerId }];
    });
    const counted = rows.filter((r) => !ignored.has(r.key));
    const wins = (teamId: string | null) =>
      teamId === null
        ? 0
        : counted.filter((r) => r.winnerId === teamId).length +
          room.manual_results.filter((m) => m.team_id === teamId).length;
    const total = (r: ResultRow, teamId: string | null) => r.totals.find((t) => t.teamId === teamId)?.score ?? 0;
    return {
      rows,
      maps: counted.map((r) => ({
        map_id: r.entry.map.id,
        team1_score: total(r, room.team1_id),
        team2_score: total(r, room.team2_id),
        players: playerScores(r.result, roster),
      })),
      outsidePool: finished.length - rows.length,
      team1: wins(room.team1_id),
      team2: wins(room.team2_id),
    };
  }, [room, finished, roster, ignored, fromSaved]);

  // Whether the site has the score and the maps it was counted from as they are here.
  const scoreSaved =
    room !== null &&
    tally !== null &&
    scoreKey(room.team1_score, room.team2_score, room.map_results) === scoreKey(tally.team1, tally.team2, tally.maps);

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
  // Set by a sync from osu!: the referee asked for those results, so they're saved
  // even if they're the first reading.
  const saveNext = useRef(false);
  const [scoreSaving, setScoreSaving] = useState(false);
  useEffect(() => {
    if (!liveScore || !room || !tally) {
      lastTally.current = null;
      return;
    }
    const key = scoreKey(tally.team1, tally.team2, tally.maps);
    const previous = lastTally.current;
    lastTally.current = key;
    const forced = saveNext.current;
    saveNext.current = false;
    if (!forced && (previous === null || previous === key)) return;
    if (scoreSaved) return;
    const { team1, team2, maps } = tally;
    setScoreSaving(true);
    setBracketScore(room.id, team1, team2, maps)
      .then(() => setRoom((r) => r && { ...r, team1_score: team1, team2_score: team2, map_results: maps }))
      .catch((e) => setError(errorMessage(e)))
      .finally(() => setScoreSaving(false));
  }, [liveScore, room, tally, scoreSaved]);

  // Reads osu!'s record of the match's rooms and counts the results from it from now on.
  const syncFromOsu = useCallback(async () => {
    setSyncing(true);
    setError(null);
    try {
      const rooms = await fetchOsuGames(matchId);
      saveNext.current = true;
      setOsuSync({ rooms, at: new Date().toISOString() });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSyncing(false);
    }
  }, [matchId]);

  // While counting from osu!, a map the chat sees finish is read from osu! again shortly
  // after, so the score keeps up without the referee syncing by hand.
  const chatFinished = lobby.results.length;
  const seenFinished = useRef(chatFinished);
  useEffect(() => {
    const grew = chatFinished > seenFinished.current;
    seenFinished.current = chatFinished;
    if (!grew || !osuSync || concluded) return;
    const timer = window.setTimeout(() => {
      fetchOsuGames(matchId)
        .then((rooms) => setOsuSync({ rooms, at: new Date().toISOString() }))
        .catch((e) => setError(errorMessage(e)));
    }, OSU_REFETCH_MS);
    return () => window.clearTimeout(timer);
  }, [chatFinished, osuSync, concluded, matchId]);

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

  // Reads everyone's mods as each map starts and puts bracket players in their slot and
  // on their team's colour as they join, also while the referee is on another match (see
  // roomAutomation.ts). Replaced rooms stop. The room as the page reads it goes along,
  // so the automation knows who's in which slot.
  const automate = canRun && !concluded;
  const autoPlace = room?.kind === "bracket" && options.autoPlace;
  const placements = useMemo<Placement[]>(
    () =>
      roster.flatMap((r) =>
        r.color ? [{ osuId: r.player.osu_id, username: r.player.username, slot: r.home, team: r.color }] : [],
      ),
    [roster],
  );
  const latestState = roomStates.length > 0 ? roomStates[roomStates.length - 1].state : EMPTY_ROOM;
  useEffect(() => {
    for (const r of rooms.slice(0, -1)) unregisterRoom(`#mp_${r.mp_id}`);
    if (!channel) return;
    if (!automate) return unregisterRoom(channel);
    registerRoom(channel, { autoPlace, players: placements, room: latestState });
  }, [rooms, channel, automate, autoPlace, placements, latestState]);
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

  // Whose pick is next: by the rules from the bans and picks, unless the referee said
  // otherwise since the last one was marked.
  const teams = useMemo<MatchTeam[]>(() => (room ? matchTeams(room) : []), [room]);
  const marks = room ? `${room.bans.length}:${room.picks.length}` : "";
  const [pickOverride, setPickOverride] = useState<{ marks: string; teamId: string } | null>(null);
  const ruledPicker = room ? nextPicker(room, teams) : null;
  const nextPick =
    pickOverride?.marks === marks ? (teams.find((t) => t.id === pickOverride.teamId) ?? ruledPicker) : ruledPicker;

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
    /**
     * Moves a player in the room to slot `to` and onto `team` (either null to leave it), swapping
     * with whoever holds the slot, then reads the room again so the page shows where they ended up.
     */
    place: (slot: RoomSlot, to: number | null, team: SlotTeam | null) => {
      const r = findRoster(slot);
      const mover = r ? { ...slot, userId: r.player.osu_id } : slot;
      const commands = moveCommands(latestState, mover, to, team);
      if (commands.length === 0) return;
      for (const line of commands) say(line);
      say("!mp settings");
    },

    /** Puts every bracket player in the room in their slot and on their team's colour. */
    fixPlacement: () => {
      const commands = placeCommands(latestState, placements, matchSlots(placements));
      if (commands.length === 0) return;
      for (const line of commands) say(line);
      say("!mp settings");
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

    /** Says whose pick is next, overriding the rules until the next ban or pick. */
    setNextPick: (teamId: string) => setPickOverride({ marks, teamId }),

    /** Tells the room the running score and whose pick is next. */
    announceScore: () => {
      if (teams.length === 2 && tally) say(scoreMessage(teams, [tally.team1, tally.team2], nextPick));
    },

    settings: () => say("!mp settings"),
    lock: (locked: boolean) => say(locked ? "!mp lock" : "!mp unlock"),
    timer: (seconds: number) => say(`!mp timer ${seconds}`),
    abortTimer: () => say("!mp aborttimer"),
    start: (seconds: number) => say(seconds > 0 ? `!mp start ${seconds}` : "!mp start"),
    abort: () => say("!mp abort"),
    close: () => say("!mp close"),
    rejoin: () => channel && join(channel),

    toggleIgnored: (key: string) =>
      setIgnored((prev) => {
        const next = new Set(prev);
        if (!next.delete(key)) next.add(key);
        return next;
      }),

    /** Counts the results from osu!'s record of the rooms, and saves them. Again to read it anew. */
    syncFromOsu,

    /** Goes back to counting the results from the rooms' chat. */
    readFromChat: () => setOsuSync(null),

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

    /** Saves the score and the maps it was counted from as they are on the page. */
    saveScore: () =>
      run(async () => {
        if (!room || !tally) return;
        await setBracketScore(room.id, tally.team1, tally.team2, tally.maps);
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
    currentEntry,
    mapStartedAt,
    teams,
    nextPick,
    roster,
    findRoster,
    options,
    ignored,
    finished,
    source,
    osuSyncedAt: osuSync?.at ?? null,
    syncing,
    tally,
    scoreSaved,
    liveScore,
    scoreSaving,
    actions,
  };
}

export type RefereeRoomState = ReturnType<typeof useRefereeRoom>;
