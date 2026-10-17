import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorMessage } from "../../api/client";
import { fetchPublicChat, fetchPublicMatch } from "../../api/matches";
import type { Player, PublicMatch, RoomChatMessage } from "../../api/types";
import type { ChatLine } from "../../irc/IrcContext";
import { useLive } from "../../live/LiveContext";
import { EMPTY_ROOM, sameName, type RoomSlot, type RoomState } from "../../lib/bancho";
import { BAN_LIMIT, entryOf, matchTeams, pickTurn, poolEntries, type MatchTeam } from "../admin/referee/pool";
import { toChatLine } from "../admin/referee/roomChat";
import {
  currentPoolEntry,
  mapStartTime,
  readLobby,
  rosterOf,
  savedTally,
  type FinishedMap,
  type RosterPlayer,
  type Tally,
} from "../admin/referee/useRefereeRoom";

// The public match page reads the same room the referee page runs, from the server
// instead of Bancho: the match as saved, and each room's chat as the referee's relay
// saved it, then every new line over the live socket. The room's state (map, players,
// mods, scores) is read from BanchoBot's lines exactly as the referee page reads them.

/** One of the match's rooms with its chat so far. */
export interface PublicRoomChat {
  mpId: number;
  url: string;
  lines: ChatLine[];
  /** The match's latest room, where it's being played. */
  current: boolean;
}

// A burst of changes (a conclusion, a ban and the score) is one reload.
const RELOAD_DELAY_MS = 250;
// How often the page reloads by itself while the live socket is down and the match is on.
const FALLBACK_MS = 15_000;

/** Adds the lines a fetch didn't have yet (they arrived live while it was out) to what it returned. */
function mergeFetched(fetched: RoomChatMessage[], had: RoomChatMessage[] | undefined): RoomChatMessage[] {
  if (!had || had.length === 0) return fetched;
  const known = new Set(fetched.map((m) => m.id));
  const extra = had.filter((m) => !known.has(m.id));
  return extra.length === 0 ? fetched : [...fetched, ...extra];
}

export function useMatchPage(matchId: string) {
  const [match, setMatch] = useState<PublicMatch | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Each room's saved lines, by room id, oldest first.
  const [chat, setChat] = useState<ReadonlyMap<number, RoomChatMessage[]>>(new Map());
  // Whether the saved chat has arrived once, so what it holds isn't counted as new.
  const [chatLoaded, setChatLoaded] = useState(false);
  const { subscribe, watch, connected, spectators: watchers } = useLive();

  const reload = useCallback(async () => {
    try {
      setMatch(await fetchPublicMatch(matchId));
      setLoadError(null);
    } catch (e) {
      setLoadError(errorMessage(e));
    }
  }, [matchId]);

  const reloadChat = useCallback(async () => {
    try {
      const rooms = await fetchPublicChat(matchId);
      setChat((prev) => new Map(rooms.map((r) => [r.mp_id, mergeFetched(r.messages, prev.get(r.mp_id))])));
      setChatLoaded(true);
    } catch {
      // The match's own reload says what's wrong; the chat comes back with the next one.
    }
  }, [matchId]);

  useEffect(() => {
    reload();
  }, [reload]);

  // The rooms' chat is loaded again whenever the match gets a room: the lines said while
  // the referee set it up came before the page knew it.
  const roomsKey = match?.rooms.map((r) => r.mp_id).join(",") ?? "";
  const loaded = match !== null;
  useEffect(() => {
    if (loaded) reloadChat();
  }, [loaded, roomsKey, reloadChat]);

  // Live: it has a room and isn't concluded. Only then is the chat streamed: the page asks
  // the server for it (again when the rooms change, since the server looks them up when
  // asked). A concluded match's chat is the saved one; an upcoming one starts streaming
  // when its room is added, which reloads the match.
  const live = match !== null && match.concluded_at === null && match.rooms.length > 0;
  useEffect(() => {
    if (!live) return;
    watch(matchId);
    return () => watch(null);
  }, [live, matchId, roomsKey, watch]);

  const streaming = useRef(live);
  streaming.current = live;
  const rooms = useRef<Set<number>>(new Set());
  rooms.current = new Set(match?.rooms.map((r) => r.mp_id) ?? []);
  useEffect(() => {
    let timer: number | undefined;
    const later = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => reload(), RELOAD_DELAY_MS);
    };
    const off = subscribe((event) => {
      switch (event.type) {
        case "chat": {
          if (!rooms.current.has(event.mp_id)) return;
          const { message } = event;
          setChat((prev) => {
            const lines = prev.get(event.mp_id) ?? [];
            if (lines.slice(-50).some((m) => m.id === message.id)) return prev;
            return new Map(prev).set(event.mp_id, [...lines, message]);
          });
          return;
        }
        case "score":
        case "match":
          if (event.match_id === matchId) later();
          return;
        case "bracket":
          // Also what a reconnect sends, so the chat missed meanwhile is fetched too (a
          // concluded match's saved chat doesn't change).
          later();
          if (streaming.current) reloadChat();
          return;
      }
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [subscribe, matchId, reload, reloadChat]);

  // While the socket is down and the match is live, the page polls instead.
  useEffect(() => {
    if (connected || !live) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "visible") return;
      reload();
      reloadChat();
    }, FALLBACK_MS);
    return () => window.clearInterval(timer);
  }, [connected, live, reload, reloadChat]);

  const roomChats = useMemo<PublicRoomChat[]>(
    () =>
      (match?.rooms ?? []).map((r, i, all) => ({
        mpId: r.mp_id,
        url: r.url,
        lines: (chat.get(r.mp_id) ?? []).map(toChatLine),
        current: i === all.length - 1,
      })),
    [match, chat],
  );

  // The latest room as BanchoBot described it, with the finished maps of every room.
  const roomStates = useMemo(() => roomChats.map((c) => ({ mpId: c.mpId, state: readLobby(c.lines) })), [roomChats]);
  const lobby = useMemo<RoomState>(() => {
    const latest = roomStates.length > 0 ? roomStates[roomStates.length - 1].state : EMPTY_ROOM;
    return { ...latest, results: roomStates.flatMap((r) => r.state.results) };
  }, [roomStates]);
  const finished = useMemo<FinishedMap[]>(
    () => roomStates.flatMap(({ mpId, state }) => state.results.map((result, n) => ({ result, key: `chat:${mpId}:${n}` }))),
    [roomStates],
  );

  const currentEntry = useMemo(() => (match ? currentPoolEntry(match, lobby) : undefined), [match, lobby]);
  const latestLines = roomChats.length > 0 ? roomChats[roomChats.length - 1].lines : null;
  const mapStartedAt = useMemo(() => mapStartTime(lobby, latestLines), [lobby, latestLines]);

  const roster = useMemo<RosterPlayer<Player>[]>(() => (match ? rosterOf(match) : []), [match]);
  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find((r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username)),
    [roster],
  );
  const teams = useMemo<MatchTeam[]>(() => (match ? matchTeams(match) : []), [match]);

  // A bracket match's sheet is what its referee saved: the counted maps with every
  // player's score, as the site and Discord show it. Its score is the saved one, which
  // the referee may have entered by hand.
  const tally = useMemo<Tally | null>(() => {
    if (!match || match.kind !== "bracket") return null;
    const saved = savedTally(match, poolEntries(match), roster);
    return {
      ...saved,
      team1: match.team1_score ?? saved.team1,
      team2: match.team2_score ?? saved.team2,
    };
  }, [match, roster]);

  // The pool maps played so far, from the saved results and the rooms' chat.
  const played = useMemo(() => {
    if (!match) return new Set<string | undefined>();
    const entries = poolEntries(match);
    const picks = new Set(match.picks.map((p) => p.map_id));
    return new Set<string | undefined>([
      ...match.map_results.map((r) => r.map_id ?? undefined),
      ...finished.map((f) => entryOf(entries, f.result, picks)?.map.id),
    ]);
  }, [match, finished]);

  // Who is watching with this tab, only while the match is live (that's when it's watched).
  const spectators = live && watchers?.match_id === matchId ? watchers : null;

  const nextPick = match && live && match.bans.length >= BAN_LIMIT ? pickTurn(match, teams, match.next_pick_team_id) : null;

  return {
    match,
    loadError,
    connected,
    roomChats,
    chatLoaded,
    lobby,
    finished,
    currentEntry,
    mapStartedAt,
    roster,
    findRoster,
    teams,
    tally,
    played,
    live,
    nextPick,
    spectators,
  };
}

export type MatchPageState = ReturnType<typeof useMatchPage>;
