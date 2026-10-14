import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ircSocketUrl } from "../api/referee";
import { useSignedInUser } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";

// The Bancho IRC connection for the whole admin area, so it survives moving between
// matches and tabs. The backend relays it over a WebSocket (handlers/irc.rs) and keeps
// one connection per user, shared by all of their browser tabs: a tab opened later
// joins it without logging in. This keeps the tab's chat history and hands every
// incoming line to listeners.

export type IrcStatus = "offline" | "connecting" | "online";

export interface ChatLine {
  id: number;
  /** Arrival time, ms. */
  at: number;
  from: string;
  text: string;
  action: boolean;
  /** Bancho refusing a command, rather than someone talking. */
  notice: boolean;
}

export interface Conversation {
  /** "#mp_123" or the other person's nick. */
  name: string;
  lines: ChatLine[];
  /** Channels only: whether we're in it right now. */
  joined: boolean;
}

export interface IncomingMessage {
  from: string;
  target: string;
  text: string;
  /** The conversation it went into. */
  conversation: string;
}

type ServerMsg =
  | { type: "connected"; nick: string; session: number }
  | { type: "login_failed" }
  | { type: "login_blocked"; retry_after: number }
  | { type: "no_session" }
  | { type: "message"; from: string; target: string; text: string; action: boolean; seq?: number }
  | { type: "past_message"; from: string; target: string; text: string; action: boolean; at: number; seq: number }
  | { type: "joined"; channel: string }
  | { type: "parted"; channel: string }
  | { type: "conversation_closed"; name: string }
  | { type: "error"; message: string }
  | { type: "closed"; reason: string };

interface IrcApi {
  status: IrcStatus;
  /** Our nick once logged in. */
  nick: string | null;
  /** Why the last connection failed or ended, if it wasn't us. */
  error: string | null;
  /** Keyed by conversationKey(). */
  conversations: Record<string, Conversation>;
  /** Logs in, or joins the connection another tab already has. */
  connect: (username: string, password: string) => void;
  /** Ends the connection for every tab. */
  disconnect: () => void;
  send: (target: string, text: string) => void;
  join: (channel: string) => void;
  part: (channel: string) => void;
  /**
   * Drops a conversation and its history; a new message starts it again. A private
   * one closes in every tab, a room only in this one.
   */
  close: (name: string) => void;
  /** Called for every incoming chat line; returns the unsubscribe. */
  onMessage: (listener: (msg: IncomingMessage) => void) => () => void;
}

const MAX_LINES = 2000;

export const conversationKey = (name: string) => name.toLowerCase();
export const isChannel = (name: string) => name.startsWith("#");

const IrcContext = createContext<IrcApi | null>(null);

// Tells this browser's other tabs that we're online, so the ones that aren't join in.
const TABS_CHANNEL = "bancho-irc";

export function IrcProvider({ children }: { children: ReactNode }) {
  const canReferee = hasRole(useSignedInUser(), "referee");
  const [status, setStatus] = useState<IrcStatus>("offline");
  const [nick, setNick] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [conversations, setConversations] = useState<Record<string, Conversation>>({});
  const socket = useRef<WebSocket | null>(null);
  const nickRef = useRef<string | null>(null);
  const listeners = useRef(new Set<(msg: IncomingMessage) => void>());
  const nextId = useRef(1);
  // Private messages from before this tab joined (the backend keeps them for the
  // connection) get negative ids, like saved room lines, so they aren't unread.
  const nextPastId = useRef(-1);
  // The connection this tab last joined and the last private message it has from it,
  // so joining it again doesn't repeat what the tab already shows.
  const seen = useRef({ session: 0, seq: 0 });
  const tabs = useRef<BroadcastChannel | null>(null);

  const drop = useCallback((name: string) => {
    setConversations((all) => {
      const rest = { ...all };
      delete rest[conversationKey(name)];
      return rest;
    });
  }, []);

  const update = useCallback((name: string, change: (c: Conversation) => Conversation) => {
    setConversations((all) => {
      const key = conversationKey(name);
      const current = all[key] ?? { name, lines: [], joined: false };
      return { ...all, [key]: change(current) };
    });
  }, []);

  const addLine = useCallback(
    (name: string, line: Omit<ChatLine, "id" | "at">, past?: { at: number }) => {
      const full = past
        ? { ...line, id: nextPastId.current--, at: past.at }
        : { ...line, id: nextId.current++, at: Date.now() };
      update(name, (c) => ({ ...c, lines: [...c.lines, full].slice(-MAX_LINES) }));
    },
    [update],
  );

  // A private message lives under the other person's name, whoever sent it.
  const conversationOf = useCallback(({ from, target }: { from: string; target: string }) => {
    const fromMe = from.toLowerCase() === nickRef.current?.toLowerCase();
    return isChannel(target) || fromMe ? target : from;
  }, []);

  const handle = useCallback(
    (msg: ServerMsg) => {
      switch (msg.type) {
        case "connected":
          if (seen.current.session !== msg.session) seen.current = { session: msg.session, seq: 0 };
          nickRef.current = msg.nick;
          setNick(msg.nick);
          setStatus("online");
          setError(null);
          tabs.current?.postMessage("online");
          break;
        case "login_failed":
          setError("Bancho girişi reddetti: kullanıcı adını ve IRC şifresini kontrol et.");
          break;
        case "login_blocked":
          setError(
            `Çok fazla hatalı giriş denemesi. ${Math.ceil(msg.retry_after / 60)} dakika sonra tekrar dene.`,
          );
          break;
        case "message": {
          if (msg.seq) seen.current.seq = msg.seq;
          const conversation = conversationOf(msg);
          addLine(conversation, { from: msg.from, text: msg.text, action: msg.action, notice: false });
          for (const listener of listeners.current) {
            listener({ from: msg.from, target: msg.target, text: msg.text, conversation });
          }
          break;
        }
        // Only shown: listeners are for what happens from now on.
        case "past_message":
          if (msg.seq <= seen.current.seq) break;
          seen.current.seq = msg.seq;
          addLine(conversationOf(msg), { from: msg.from, text: msg.text, action: msg.action, notice: false }, msg);
          break;
        case "joined":
          update(msg.channel, (c) => ({ ...c, joined: true }));
          break;
        case "conversation_closed":
          drop(msg.name);
          break;
        case "parted":
          update(msg.channel, (c) => ({ ...c, joined: false }));
          break;
        case "error": {
          // "#mp_1: No such channel #mp_1" goes into the conversation it's about.
          const subject = msg.message.split(":")[0];
          addLine(subject, { from: "Bancho", text: msg.message, action: false, notice: true });
          break;
        }
        case "closed":
          setError((e) => e ?? `Bağlantı kapandı: ${msg.reason}`);
          break;
      }
    },
    [addLine, conversationOf, drop, update],
  );

  // Closes this tab's socket; the connection lives on while other tabs use it.
  const leave = useCallback(() => {
    const ws = socket.current;
    socket.current = null;
    ws?.close();
    nickRef.current = null;
    setNick(null);
    setStatus("offline");
    setConversations((all) =>
      Object.fromEntries(Object.entries(all).map(([k, c]) => [k, { ...c, joined: false }])),
    );
  }, []);

  const open = useCallback(
    (first: object) => {
      socket.current?.close();
      setError(null);
      setStatus("connecting");
      const ws = new WebSocket(ircSocketUrl());
      socket.current = ws;
      ws.onopen = () => ws.send(JSON.stringify(first));
      ws.onmessage = (e) => {
        if (socket.current !== ws) return;
        let msg: ServerMsg;
        try {
          msg = JSON.parse(e.data as string) as ServerMsg;
        } catch {
          return; // not ours
        }
        // Resuming found nothing to join: just offline, not an error.
        if (msg.type === "no_session") leave();
        else handle(msg);
      };
      ws.onclose = () => {
        if (socket.current !== ws) return;
        setError((e) => e ?? "Bancho bağlantısı kurulamadı.");
        leave();
      };
    },
    [leave, handle],
  );

  const connect = useCallback(
    (username: string, password: string) => open({ type: "login", username, password }),
    [open],
  );
  const resume = useCallback(() => open({ type: "resume" }), [open]);

  const disconnect = useCallback(() => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: "quit" }));
    leave();
  }, [leave]);

  // Joins the connection another tab already has, now and whenever one comes online.
  useEffect(() => {
    if (!canReferee) return;
    resume();
    if (typeof BroadcastChannel !== "undefined") {
      const channel = new BroadcastChannel(TABS_CHANNEL);
      channel.onmessage = () => {
        if (!socket.current) resume();
      };
      tabs.current = channel;
    }
    return () => {
      tabs.current?.close();
      tabs.current = null;
      const ws = socket.current;
      socket.current = null;
      ws?.close();
    };
  }, [canReferee, resume]);

  const sendJson = useCallback((msg: object) => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify(msg));
  }, []);

  const send = useCallback((target: string, text: string) => sendJson({ type: "send", target, text }), [sendJson]);
  const join = useCallback((channel: string) => sendJson({ type: "join", channel }), [sendJson]);
  const part = useCallback((channel: string) => sendJson({ type: "part", channel }), [sendJson]);
  const close = useCallback(
    (name: string) => {
      drop(name);
      if (!isChannel(name)) sendJson({ type: "close", name });
    },
    [drop, sendJson],
  );
  const onMessage = useCallback((listener: (msg: IncomingMessage) => void) => {
    listeners.current.add(listener);
    return () => {
      listeners.current.delete(listener);
    };
  }, []);

  const api = useMemo<IrcApi>(
    () => ({ status, nick, error, conversations, connect, disconnect, send, join, part, close, onMessage }),
    [status, nick, error, conversations, connect, disconnect, send, join, part, close, onMessage],
  );

  return <IrcContext.Provider value={api}>{children}</IrcContext.Provider>;
}

export function useIrc(): IrcApi {
  const api = useContext(IrcContext);
  if (!api) throw new Error("useIrc needs an IrcProvider");
  return api;
}
