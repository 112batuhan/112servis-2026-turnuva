import { useEffect, useState } from "react";

// tosu (https://github.com/tosuapp/tosu) reads the osu! tournament client's memory on the
// streamer's machine. The overlays use it for that alone: each spectator client's live score and
// the tourney manager's state, which says when the clients are playing and when every one of them
// is on the results screen (which also tells the control dock when to switch pages). Everything
// else comes from the site. tosu refuses a WebSocket from a page whose origin isn't in its
// ALLOWED_IPS (Settings → Allowed IPs), so the site's hostname has to be added there; a refused
// socket just closes, which reads as unreachable.

/** osu!'s TourneyState, as the tourney manager reports it (tourney.ipcState). */
export const IPC = {
  initialising: 0,
  idle: 1,
  waitingForClients: 2,
  /** The spectator clients are playing a map. */
  playing: 3,
  /** Every spectator client is on the results screen. */
  ranking: 4,
} as const;

export interface TosuClient {
  ipcId: number;
  /** tosu's guess: the first half of the clients by ipcId is left. */
  team: "left" | "right";
  /** The osu! user the client spectates, 0 while it has none. */
  userId: number;
  userName: string;
  score: number;
}

/** Whether tosu is there and sees a tournament client. */
export type TosuReport = "connected" | "connecting" | "unreachable" | "no_tourney";

export interface TosuTourney {
  status: TosuReport;
  ipcState: number;
  clients: TosuClient[];
}

export const DEFAULT_TOSU_ADDR = "127.0.0.1:24050";
const RETRY_MS = 3_000;

const EMPTY: TosuTourney = { status: "connecting", ipcState: 0, clients: [] };

interface RawClient {
  ipcId: number;
  team: "left" | "right";
  user?: { id?: number; name?: string };
  play?: { score?: number };
}

interface RawMessage {
  tourney?: { ipcState?: number; clients?: RawClient[] };
}

function read(msg: RawMessage): TosuTourney {
  const t = msg.tourney;
  const raw = t?.clients ?? [];
  const clients = raw.map((c) => ({
    ipcId: c.ipcId,
    team: c.team,
    userId: c.user?.id ?? 0,
    userName: c.user?.name ?? "",
    score: c.play?.score ?? 0,
  }));
  return {
    status: clients.length > 0 ? "connected" : "no_tourney",
    ipcState: t?.ipcState ?? 0,
    clients,
  };
}

function same(a: TosuTourney, b: TosuTourney): boolean {
  return (
    a.status === b.status &&
    a.ipcState === b.ipcState &&
    a.clients.length === b.clients.length &&
    a.clients.every((c, i) => {
      const d = b.clients[i];
      return c.ipcId === d.ipcId && c.team === d.team && c.userId === d.userId && c.score === d.score;
    })
  );
}

/** The tournament client as tosu reads it, reconnecting on its own. `addr` is "host:port". */
export function useTosu(addr: string | null): TosuTourney {
  const [state, setState] = useState<TosuTourney>(EMPTY);
  useEffect(() => {
    let closed = false;
    let timer: number | undefined;
    let ws: WebSocket | null = null;
    const connect = () => {
      let opened = false;
      ws = new WebSocket(`ws://${addr || DEFAULT_TOSU_ADDR}/websocket/v2`);
      ws.onopen = () => {
        opened = true;
      };
      ws.onmessage = (e) => {
        let next: TosuTourney;
        try {
          next = read(JSON.parse(e.data as string) as RawMessage);
        } catch {
          return;
        }
        setState((prev) => (same(prev, next) ? prev : next));
      };
      ws.onclose = () => {
        if (closed) return;
        // Never opened: tosu isn't running, or it refused this page's origin (ALLOWED_IPS).
        setState((prev) => ({ ...(opened ? prev : EMPTY), status: "unreachable" }));
        timer = window.setTimeout(connect, RETRY_MS);
      };
    };
    connect();
    return () => {
      closed = true;
      window.clearTimeout(timer);
      ws?.close();
    };
  }, [addr]);
  return state;
}
