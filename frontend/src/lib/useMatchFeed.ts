import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "../api/client";
import type { LiveRoom, RoomChat } from "../api/types";
import { RELOAD_DELAY_MS, useLive } from "../live/LiveContext";
import { appendLine, mergeFetched, type StoredLine } from "./roomChat";

// What the referee page and the public match page have in common about following a match: the
// match as the server has it, its room as the bots read it (the same reading on every page) and
// each room's chat as its bot saved it, loaded once and then kept current over the live socket;
// while the socket is down, the page reloads by itself instead.

/** What a match page reads: a match with its rooms and the bots' reading of the room. */
interface FeedMatch {
  rooms: { mp_id: number }[];
  live: LiveRoom | null;
}

interface Feed<M extends FeedMatch> {
  fetchMatch: (matchId: string) => Promise<M>;
  fetchChat: (matchId: string) => Promise<RoomChat[]>;
  /**
   * Whether the match is followed over the socket now: the page asks the server for its room and
   * its chat as they change (again whenever its rooms change, since the server looks them up when
   * asked).
   */
  follows: (match: M) => boolean;
  /** Whether the tab counts among the match's spectators while it follows it. */
  spectate: boolean;
  /** A staff page: it also reloads when something only the referee page shows changes. */
  staff: boolean;
  /** How often the page reloads by itself while the socket is down. */
  pollMs: number;
  /** Whether it does so even when the match isn't followed. */
  pollAlways: boolean;
  /** Told when the saved chat couldn't be loaded; the match's own reload reports itself. */
  onChatError?: (message: string) => void;
}

export function useMatchFeed<M extends FeedMatch>(matchId: string, feed: Feed<M>) {
  const { fetchMatch, fetchChat, spectate, staff, pollMs, pollAlways } = feed;
  const [match, setMatch] = useState<M | null>(null);
  const [live, setLive] = useState<LiveRoom | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Each room's saved lines, by room id, oldest first.
  const [chat, setChat] = useState<ReadonlyMap<number, StoredLine[]>>(new Map());
  // Whether the saved chat has arrived once, so what it holds isn't counted as new.
  const [chatLoaded, setChatLoaded] = useState(false);
  const { subscribe, watch, connected } = useLive();
  const onChatError = useRef(feed.onChatError);
  onChatError.current = feed.onChatError;

  // A burst of changes sets off several reloads, and their answers can come back out of order:
  // an answer older than one already shown is dropped. The room's reading also arrives over the
  // socket, so an answer doesn't replace a reading that came in while it was on its way (if the
  // room changed again since, the socket says so).
  const requests = useRef({ started: 0, shown: 0, readings: 0 });

  const reload = useCallback(async () => {
    const r = requests.current;
    const mine = ++r.started;
    const readings = r.readings;
    try {
      const m = await fetchMatch(matchId);
      if (mine < r.shown) return;
      r.shown = mine;
      setMatch(m);
      if (r.readings === readings) setLive(m.live);
      setLoadError(null);
    } catch (e) {
      if (mine < r.shown) return;
      setLoadError(errorMessage(e));
    }
  }, [fetchMatch, matchId]);

  const reloadChat = useCallback(async () => {
    try {
      const rooms = await fetchChat(matchId);
      setChat((prev) => new Map(rooms.map((r) => [r.mp_id, mergeFetched(r.messages, prev.get(r.mp_id))])));
      setChatLoaded(true);
    } catch (e) {
      onChatError.current?.(errorMessage(e));
    }
  }, [fetchChat, matchId]);

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

  const following = match !== null && feed.follows(match);
  useEffect(() => {
    if (!following) return;
    watch(matchId, spectate);
    return () => watch(null);
  }, [following, matchId, roomsKey, watch, spectate]);

  const followingNow = useRef(following);
  followingNow.current = following;
  const roomIds = useRef<Set<number>>(new Set());
  roomIds.current = new Set(match?.rooms.map((r) => r.mp_id) ?? []);
  useEffect(() => {
    let timer: number | undefined;
    // A burst of changes (a conclusion, a ban and the score it moves) is one reload.
    const later = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(reload, RELOAD_DELAY_MS);
    };
    const off = subscribe((event) => {
      if (event.type === "chat") {
        if (!roomIds.current.has(event.mp_id)) return;
        setChat((prev) => new Map(prev).set(event.mp_id, appendLine(prev.get(event.mp_id) ?? [], event.message)));
      } else if (event.type === "room") {
        if (event.match_id !== matchId) return;
        requests.current.readings += 1;
        setLive(event.room);
      } else if (event.type === "score" || event.type === "match" || (staff && event.type === "referee")) {
        if (event.match_id === matchId) later();
      } else if (event.type === "bracket") {
        // Rooms, staff, conclusions; also what a reconnect sends, so the chat missed meanwhile is
        // loaded too (the saved chat of a match that isn't followed doesn't change).
        later();
        if (followingNow.current) reloadChat();
      }
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [subscribe, matchId, staff, reload, reloadChat]);

  const polls = !connected && (pollAlways || following);
  useEffect(() => {
    if (!polls) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "visible") return;
      reload();
      reloadChat();
    }, pollMs);
    return () => window.clearInterval(timer);
  }, [polls, pollMs, reload, reloadChat]);

  return { match, setMatch, live, loadError, chat, chatLoaded, reload, reloadChat, connected, following };
}
