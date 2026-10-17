import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorMessage } from "../../api/client";
import { fetchPublicChat, fetchPublicMatch } from "../../api/matches";
import type { LiveRoom, Player, PublicMatch } from "../../api/types";
import type { ChatLine } from "../../irc/IrcContext";
import { sameName, type RoomSlot } from "../../lib/bancho";
import { raceOf, type Race } from "../../lib/bestOf";
import {
  currentEntryOf,
  finishedOf,
  lobbyOf,
  rosterOf,
  savedTally,
  type FinishedMap,
  type RosterPlayer,
  type Tally,
} from "../admin/referee/liveRoom";
import { matchTeams, poolEntries, type MatchTeam } from "../admin/referee/pool";
import { appendLine, mergeFetched, toChatLine, type StoredLine } from "../admin/referee/roomChat";
import { useLive } from "../../live/LiveContext";

// The public match page reads the same room the referee page runs: the match as saved, the room
// as its bot reads it (the same reading on every page), and each room's chat as the bot saved it,
// then every change over the live socket.

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

export function useMatchPage(matchId: string) {
  const [match, setMatch] = useState<PublicMatch | null>(null);
  const [live, setLive] = useState<LiveRoom | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Each room's saved lines, by room id, oldest first.
  const [chat, setChat] = useState<ReadonlyMap<number, StoredLine[]>>(new Map());
  // Whether the saved chat has arrived once, so what it holds isn't counted as new.
  const [chatLoaded, setChatLoaded] = useState(false);
  const { subscribe, watch, connected, spectators: watchers } = useLive();

  const reload = useCallback(async () => {
    try {
      const m = await fetchPublicMatch(matchId);
      setMatch(m);
      setLive(m.live);
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

  // The rooms' chat is loaded again whenever the match gets a room: the lines said while the bot
  // set it up came before the page knew it.
  const roomsKey = match?.rooms.map((r) => r.mp_id).join(",") ?? "";
  const loaded = match !== null;
  useEffect(() => {
    if (loaded) reloadChat();
  }, [loaded, roomsKey, reloadChat]);

  // Live: it has a room and isn't concluded. Only then is it followed: the page asks the server for
  // it (again when the rooms change, since the server looks them up when asked). A concluded
  // match's chat is the saved one; an upcoming one is followed once its room is added, which
  // reloads the match.
  const isLive = match !== null && match.concluded_at === null && match.rooms.length > 0;
  useEffect(() => {
    if (!isLive) return;
    watch(matchId);
    return () => watch(null);
  }, [isLive, matchId, roomsKey, watch]);

  const streaming = useRef(isLive);
  streaming.current = isLive;
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
        case "chat":
          if (!rooms.current.has(event.mp_id)) return;
          setChat((prev) => new Map(prev).set(event.mp_id, appendLine(prev.get(event.mp_id) ?? [], event.message)));
          return;
        case "room":
          if (event.match_id === matchId) setLive(event.room);
          return;
        case "score":
        case "match":
          if (event.match_id === matchId) later();
          return;
        case "bracket":
          // Also what a reconnect sends, so the chat missed meanwhile is fetched too (a concluded
          // match's saved chat doesn't change).
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
    if (connected || !isLive) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "visible") return;
      reload();
      reloadChat();
    }, FALLBACK_MS);
    return () => window.clearInterval(timer);
  }, [connected, isLive, reload, reloadChat]);

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

  const entries = useMemo(() => (match ? poolEntries(match) : []), [match]);
  const lobby = useMemo(() => lobbyOf(live), [live]);
  const finished = useMemo<FinishedMap[]>(() => finishedOf(live), [live]);
  const currentEntry = useMemo(() => currentEntryOf(live, entries), [live, entries]);

  const roster = useMemo<RosterPlayer<Player>[]>(() => (match ? rosterOf(match) : []), [match]);
  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find((r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username)),
    [roster],
  );
  const teams = useMemo<MatchTeam[]>(() => (match ? matchTeams(match) : []), [match]);

  // A bracket match's sheet is what the site saved: the counted maps with every player's score,
  // as Discord shows it (the room's bot saves it as each map finishes). Its score is the saved
  // one, which a host may have entered by hand.
  const tally = useMemo<Tally | null>(() => {
    if (!match || match.kind !== "bracket") return null;
    const saved = savedTally(match, entries, roster);
    return { ...saved, team1: match.team1_score ?? saved.team1, team2: match.team2_score ?? saved.team2 };
  }, [match, entries, roster]);

  // Where the saved score stands against the stage's best of (the tiebreaker, a decided race).
  const bestOf = match?.kind === "bracket" ? (match.stage?.best_of ?? null) : null;
  const race = useMemo<Race | null>(
    () =>
      bestOf !== null && tally && !match?.forfeit_slot && match?.concluded_at === null
        ? raceOf(bestOf, tally.team1, tally.team2)
        : null,
    [bestOf, tally, match?.forfeit_slot, match?.concluded_at],
  );

  // The pool maps played so far, from the saved results and the rooms.
  const played = useMemo(
    () =>
      new Set<string | undefined>([
        ...(match?.map_results ?? []).map((r) => r.map_id ?? undefined),
        ...finished.map((f) => f.mapId ?? undefined),
      ]),
    [match, finished],
  );

  // Who is watching with this tab, only while the match is live (that's when it's watched).
  const spectators = isLive && watchers?.match_id === matchId ? watchers : null;
  const nextPick = isLive ? (teams.find((t) => t.id === live?.next_pick_team_id) ?? null) : null;

  return {
    match,
    loadError,
    connected,
    roomChats,
    chatLoaded,
    lobby,
    finished,
    currentEntry,
    mapStartedAt: lobby.mapStartedAt,
    roster,
    findRoster,
    teams,
    tally,
    played,
    live: isLive,
    nextPick,
    race,
    spectators,
  };
}

export type MatchPageState = ReturnType<typeof useMatchPage>;
