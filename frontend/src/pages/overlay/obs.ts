import { useCallback, useEffect, useRef, useState } from "react";

// obs-websocket v5 (built into OBS 28+, Tools → WebSocket Server Settings), spoken by the
// overlay and the control dock. It's the control channel between them: the dock broadcasts what
// to show with BroadcastCustomEvent and the overlay hears it as a CustomEvent, all inside the
// streamer's own OBS, so the site never has to know who is streaming. WebSockets to 127.0.0.1 are
// exempt from OBS 33's local network access checks, which block fetch from a hosted page to
// localhost.

export const DEFAULT_OBS_URL = "ws://127.0.0.1:4455";

export type ObsStatus =
  /** Trying to reach OBS. */
  | "connecting"
  /** Identified: requests and events flow. */
  | "connected"
  /** OBS refused the password (or wants one). */
  | "auth_failed"
  /** Nothing answered; it retries. */
  | "offline";

/** The General event subscription (obs-websocket's EventSubscription), which carries CustomEvent. */
export const SUB_GENERAL = 1 << 0;

interface Pending {
  resolve: (data: unknown) => void;
  reject: (err: Error) => void;
}

const RETRY_MIN_MS = 1_000;
const RETRY_MAX_MS = 15_000;
// 4009: Identify's authentication was wrong.
const CLOSE_AUTH_FAILED = 4009;

async function sha256Base64(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  let binary = "";
  for (const byte of new Uint8Array(digest)) binary += String.fromCharCode(byte);
  return btoa(binary);
}

/** base64(sha256(base64(sha256(password + salt)) + challenge)), as the protocol has it. */
async function authString(password: string, salt: string, challenge: string): Promise<string> {
  return sha256Base64((await sha256Base64(password + salt)) + challenge);
}

export interface Obs {
  status: ObsStatus;
  /** Sends a request and resolves with its responseData; rejects when OBS says it failed. */
  request: <T = unknown>(requestType: string, requestData?: Record<string, unknown>) => Promise<T>;
  /** Calls `handler` with every event OBS sends; returns the unsubscribe. */
  onEvent: (handler: (type: string, data: Record<string, unknown>) => void) => () => void;
}

/**
 * One identified connection to OBS, reconnecting on its own. `url` null stays disconnected.
 * `onIdentified` runs every time the connection is (re)established.
 */
export function useObs(url: string | null, password: string, subscriptions: number, onIdentified?: () => void): Obs {
  const [status, setStatus] = useState<ObsStatus>("connecting");
  const socket = useRef<WebSocket | null>(null);
  const pending = useRef(new Map<string, Pending>());
  const handlers = useRef(new Set<(type: string, data: Record<string, unknown>) => void>());
  const identified = useRef(onIdentified);
  identified.current = onIdentified;

  useEffect(() => {
    if (!url) {
      setStatus("offline");
      return;
    }
    let closed = false;
    let retry = RETRY_MIN_MS;
    let timer: number | undefined;

    const failAll = () => {
      for (const p of pending.current.values()) p.reject(new Error("OBS bağlantısı koptu"));
      pending.current.clear();
    };

    const connect = () => {
      setStatus((s) => (s === "auth_failed" ? s : "connecting"));
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch {
        setStatus("offline");
        return;
      }
      socket.current = ws;
      ws.onmessage = async (e) => {
        let msg: { op: number; d: Record<string, unknown> };
        try {
          msg = JSON.parse(e.data as string);
        } catch {
          return;
        }
        const d = msg.d ?? {};
        if (msg.op === 0) {
          // Hello: answer with Identify, signed when OBS asks for a password.
          const auth = d.authentication as { challenge: string; salt: string } | undefined;
          const identify: Record<string, unknown> = { rpcVersion: 1, eventSubscriptions: subscriptions };
          if (auth) identify.authentication = await authString(password, auth.salt, auth.challenge);
          ws.send(JSON.stringify({ op: 1, d: identify }));
        } else if (msg.op === 2) {
          retry = RETRY_MIN_MS;
          setStatus("connected");
          identified.current?.();
        } else if (msg.op === 5) {
          const type = d.eventType as string;
          const data = (d.eventData as Record<string, unknown>) ?? {};
          for (const h of handlers.current) h(type, data);
        } else if (msg.op === 7) {
          const id = d.requestId as string;
          const p = pending.current.get(id);
          if (!p) return;
          pending.current.delete(id);
          const reqStatus = d.requestStatus as { result: boolean; comment?: string };
          if (reqStatus?.result) p.resolve(d.responseData ?? {});
          else p.reject(new Error(reqStatus?.comment ?? "OBS isteği reddetti"));
        }
      };
      ws.onclose = (e) => {
        if (socket.current === ws) socket.current = null;
        failAll();
        if (closed) return;
        if (e.code === CLOSE_AUTH_FAILED) {
          // A wrong password won't get better by retrying; a new one remounts this.
          setStatus("auth_failed");
          return;
        }
        setStatus("offline");
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
      failAll();
    };
  }, [url, password, subscriptions]);

  const request = useCallback(<T,>(requestType: string, requestData?: Record<string, unknown>) => {
    const ws = socket.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return Promise.reject(new Error("OBS'e bağlı değil"));
    const requestId = crypto.randomUUID();
    return new Promise<T>((resolve, reject) => {
      pending.current.set(requestId, { resolve: resolve as (d: unknown) => void, reject });
      ws.send(JSON.stringify({ op: 6, d: { requestType, requestId, requestData } }));
    });
  }, []);

  const onEvent = useCallback((handler: (type: string, data: Record<string, unknown>) => void) => {
    handlers.current.add(handler);
    return () => {
      handlers.current.delete(handler);
    };
  }, []);

  return { status, request, onEvent };
}
