import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { liveSocketUrl } from "../api/live";
import type { LiveRoom, RefereeViewer, RoomChatMessage } from "../api/types";
import { useAuth } from "../auth/AuthContext";

// One WebSocket per tab to the server's live hub (handlers/live.rs): score changes as
// they're saved, a signal to reload the bracket when anything else about it changes,
// and, for staff, who has each match's referee page open and what changed on it; a signed-in
// user's tabs also hear when their notifications change. Pages subscribe to the
// events; the referee page says which match it shows, and the match pages which match they
// follow (`watch`): its room as the bots read it and its chat, as they change. The public
// match page is counted among that match's spectators and is sent who else is watching. It reconnects on its own and
// again whenever the signed-in user changes, since the server reads who it is once.

export type LiveEvent =
  | { type: "score"; match_id: string; team1_score: number | null; team2_score: number | null }
  | { type: "bracket" }
  /** Something about one match the bracket doesn't show changed (bans, picks, a lobby's teams). */
  | { type: "match"; match_id: string }
  /** Staff: something only the referee page shows changed (room options, "Sayma", an osu! sync). */
  | { type: "referee"; match_id: string }
  /** A line said in a room of the watched match, as its bot saved it. */
  | { type: "chat"; mp_id: number; message: RoomChatMessage }
  /** The watched match's room as its bot reads it now. */
  | { type: "room"; match_id: string; room: LiveRoom }
  /** Staff: a bot connected, dropped, or took or left a room. */
  | { type: "bancho" }
  /** The signed-in user's notifications changed (one arrived, was read or answered, or was replaced). */
  | { type: "notifications" };

type ServerMsg =
  | LiveEvent
  | { type: "presence"; match_id: string; viewers: RefereeViewer[] }
  | { type: "presence_all"; matches: Record<string, RefereeViewer[]> }
  | ({ type: "spectators" } & Spectators);

/** Who has the watched match's public page open: signed-in users by name, each once, and the tabs nobody is signed in on. */
export interface Spectators {
  match_id: string;
  viewers: RefereeViewer[];
  guests: number;
}

interface LiveState {
  /** The socket is open, so pages don't need to poll. */
  connected: boolean;
  /** Who has each match's referee page open, by match id. Staff only; empty for everyone else. */
  presence: ReadonlyMap<string, RefereeViewer[]>;
  /** Who is watching the match this tab watches, this tab included; null until the server says. */
  spectators: Spectators | null;
  /** Calls `handler` with every score and bracket event; returns the unsubscribe. */
  subscribe: (handler: (event: LiveEvent) => void) => () => void;
  /** Tells the server this tab shows the match's referee page (or none). */
  setViewing: (matchId: string | null) => void;
  /**
   * Asks for the match's room and its chat (or none). Again whenever its rooms change, since the
   * server looks them up when asked. The tab counts among the match's spectators unless
   * `spectate` is false (the referee page).
   */
  watch: (matchId: string | null, spectate?: boolean) => void;
}

const LiveContext = createContext<LiveState | null>(null);

const RETRY_MIN_MS = 1_000;
const RETRY_MAX_MS = 30_000;

export function LiveProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const [connected, setConnected] = useState(false);
  const [presence, setPresence] = useState<ReadonlyMap<string, RefereeViewer[]>>(new Map());
  const [spectators, setSpectators] = useState<Spectators | null>(null);
  const handlers = useRef(new Set<(event: LiveEvent) => void>());
  const socket = useRef<WebSocket | null>(null);
  const viewing = useRef<string | null>(null);
  const watching = useRef<string | null>(null);
  const spectating = useRef(true);

  const sendViewing = useCallback(() => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: "viewing", match_id: viewing.current }));
  }, []);
  const sendWatch = useCallback(() => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: "watch", match_id: watching.current, spectate: spectating.current }));
  }, []);

  // A page kept in the browser's back/forward cache would keep its socket, and with it
  // its place on a referee page, so the socket closes as the page is left and a new one
  // opens if the page comes back from that cache.
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    const onHide = () => socket.current?.close();
    const onShow = (e: PageTransitionEvent) => {
      if (e.persisted) setGeneration((g) => g + 1);
    };
    window.addEventListener("pagehide", onHide);
    window.addEventListener("pageshow", onShow);
    return () => {
      window.removeEventListener("pagehide", onHide);
      window.removeEventListener("pageshow", onShow);
    };
  }, []);

  // Waits for the session check, so a signed-in user doesn't connect twice.
  const userKey = user === undefined ? null : (user?.id ?? "guest");
  useEffect(() => {
    if (userKey === null) return;
    let closed = false;
    let retry = RETRY_MIN_MS;
    // After a dropped connection, pages reload whatever they missed meanwhile.
    let reconnecting = false;
    let timer: number | undefined;

    const connect = () => {
      const ws = new WebSocket(liveSocketUrl());
      socket.current = ws;
      ws.onopen = () => {
        retry = RETRY_MIN_MS;
        setConnected(true);
        if (viewing.current) sendViewing();
        if (watching.current) sendWatch();
        if (reconnecting) for (const handler of handlers.current) handler({ type: "bracket" });
      };
      ws.onmessage = (e) => {
        let msg: ServerMsg;
        try {
          msg = JSON.parse(e.data as string) as ServerMsg;
        } catch {
          return;
        }
        if (msg.type === "presence_all") {
          setPresence(new Map(Object.entries(msg.matches)));
        } else if (msg.type === "presence") {
          setPresence((prev) => {
            const next = new Map(prev);
            if (msg.viewers.length > 0) next.set(msg.match_id, msg.viewers);
            else next.delete(msg.match_id);
            return next;
          });
        } else if (msg.type === "spectators") {
          // A list for a match this tab has since left is dropped.
          if (msg.match_id === watching.current) setSpectators({ match_id: msg.match_id, viewers: msg.viewers, guests: msg.guests });
        } else {
          for (const handler of handlers.current) handler(msg);
        }
      };
      ws.onclose = () => {
        if (socket.current === ws) socket.current = null;
        setConnected(false);
        setPresence(new Map());
        setSpectators(null);
        if (closed) return;
        reconnecting = true;
        timer = window.setTimeout(connect, retry);
        retry = Math.min(retry * 2, RETRY_MAX_MS);
      };
    };
    connect();
    return () => {
      closed = true;
      window.clearTimeout(timer);
      socket.current?.close();
      socket.current = null;
    };
  }, [userKey, generation, sendViewing, sendWatch]);

  const subscribe = useCallback((handler: (event: LiveEvent) => void) => {
    handlers.current.add(handler);
    return () => {
      handlers.current.delete(handler);
    };
  }, []);

  const setViewing = useCallback(
    (matchId: string | null) => {
      viewing.current = matchId;
      sendViewing();
    },
    [sendViewing],
  );

  const watch = useCallback(
    (matchId: string | null, spectate = true) => {
      if (matchId !== watching.current || !spectate) setSpectators(null);
      watching.current = matchId;
      spectating.current = spectate;
      sendWatch();
    },
    [sendWatch],
  );

  const value = useMemo(
    () => ({ connected, presence, spectators, subscribe, setViewing, watch }),
    [connected, presence, spectators, subscribe, setViewing, watch],
  );
  return <LiveContext.Provider value={value}>{children}</LiveContext.Provider>;
}

export function useLive(): LiveState {
  const ctx = useContext(LiveContext);
  if (!ctx) throw new Error("useLive must be used inside LiveProvider");
  return ctx;
}

/**
 * How long a burst of changes (a conclusion moves teams into two matches; a ban and the score it
 * moves) is gathered into one reload, on every page that reloads after live events.
 */
export const RELOAD_DELAY_MS = 250;

/**
 * Keeps a page's bracket in step: `onScore` applies a saved score at once, `onChanged` reloads
 * the bracket after anything else changed (gathered over a short moment).
 */
export function useLiveBracket(onScore: (event: Extract<LiveEvent, { type: "score" }>) => void, onChanged: () => void) {
  const { subscribe } = useLive();
  const callbacks = useRef({ onScore, onChanged });
  callbacks.current = { onScore, onChanged };
  useEffect(() => {
    let timer: number | undefined;
    const off = subscribe((event) => {
      if (event.type === "score") {
        callbacks.current.onScore(event);
        return;
      }
      // The others don't change what the bracket shows.
      if (event.type !== "bracket") return;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => callbacks.current.onChanged(), RELOAD_DELAY_MS);
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [subscribe]);
}
