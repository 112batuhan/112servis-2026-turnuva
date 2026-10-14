import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ircSocketUrl } from "../api/referee";

// One Bancho IRC connection for the whole admin area, so it survives moving between
// matches and tabs. The backend relays it over a WebSocket (handlers/irc.rs); this
// keeps the chat history and hands every incoming line to listeners.

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
  | { type: "connected"; nick: string }
  | { type: "login_failed" }
  | { type: "message"; from: string; target: string; text: string; action: boolean }
  | { type: "joined"; channel: string }
  | { type: "parted"; channel: string }
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
  connect: (username: string, password: string) => void;
  disconnect: () => void;
  send: (target: string, text: string) => void;
  join: (channel: string) => void;
  part: (channel: string) => void;
  /** Drops a conversation and its history; a new message starts it again. */
  close: (name: string) => void;
  /** Called for every incoming chat line; returns the unsubscribe. */
  onMessage: (listener: (msg: IncomingMessage) => void) => () => void;
}

const MAX_LINES = 2000;

export const conversationKey = (name: string) => name.toLowerCase();
export const isChannel = (name: string) => name.startsWith("#");

const IrcContext = createContext<IrcApi | null>(null);

export function IrcProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<IrcStatus>("offline");
  const [nick, setNick] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [conversations, setConversations] = useState<Record<string, Conversation>>({});
  const socket = useRef<WebSocket | null>(null);
  const nickRef = useRef<string | null>(null);
  const listeners = useRef(new Set<(msg: IncomingMessage) => void>());
  const nextId = useRef(1);

  const update = useCallback((name: string, change: (c: Conversation) => Conversation) => {
    setConversations((all) => {
      const key = conversationKey(name);
      const current = all[key] ?? { name, lines: [], joined: false };
      return { ...all, [key]: change(current) };
    });
  }, []);

  const addLine = useCallback(
    (name: string, line: Omit<ChatLine, "id" | "at">) => {
      const full = { ...line, id: nextId.current++, at: Date.now() };
      update(name, (c) => ({ ...c, lines: [...c.lines, full].slice(-MAX_LINES) }));
    },
    [update],
  );

  const handle = useCallback(
    (msg: ServerMsg) => {
      switch (msg.type) {
        case "connected":
          nickRef.current = msg.nick;
          setNick(msg.nick);
          setStatus("online");
          setError(null);
          break;
        case "login_failed":
          setError("Bancho girişi reddetti: kullanıcı adını ve IRC şifresini kontrol et.");
          break;
        case "message": {
          const fromMe = msg.from.toLowerCase() === nickRef.current?.toLowerCase();
          // A private message lives under the other person's name, whoever sent it.
          const conversation = isChannel(msg.target) ? msg.target : fromMe ? msg.target : msg.from;
          addLine(conversation, { from: msg.from, text: msg.text, action: msg.action, notice: false });
          for (const listener of listeners.current) {
            listener({ from: msg.from, target: msg.target, text: msg.text, conversation });
          }
          break;
        }
        case "joined":
          update(msg.channel, (c) => ({ ...c, joined: true }));
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
    [addLine, update],
  );

  const disconnect = useCallback(() => {
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

  const connect = useCallback(
    (username: string, password: string) => {
      socket.current?.close();
      setError(null);
      setStatus("connecting");
      const ws = new WebSocket(ircSocketUrl());
      socket.current = ws;
      ws.onopen = () => ws.send(JSON.stringify({ type: "login", username, password }));
      ws.onmessage = (e) => {
        if (socket.current !== ws) return;
        try {
          handle(JSON.parse(e.data as string) as ServerMsg);
        } catch {
          // not ours; ignore
        }
      };
      ws.onclose = () => {
        if (socket.current !== ws) return;
        setError((e) => e ?? "Bancho bağlantısı kurulamadı.");
        disconnect();
      };
    },
    [disconnect, handle],
  );

  useEffect(() => () => socket.current?.close(), []);

  const sendJson = useCallback((msg: object) => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify(msg));
  }, []);

  const send = useCallback((target: string, text: string) => sendJson({ type: "send", target, text }), [sendJson]);
  const join = useCallback((channel: string) => sendJson({ type: "join", channel }), [sendJson]);
  const part = useCallback((channel: string) => sendJson({ type: "part", channel }), [sendJson]);
  const close = useCallback((name: string) => {
    setConversations((all) => {
      const rest = { ...all };
      delete rest[conversationKey(name)];
      return rest;
    });
  }, []);
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
