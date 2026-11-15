import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { ircSocketUrl } from "../api/referee";
import { useSignedInUser } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";

// The referee's own Bancho IRC connection for the whole admin area, so it survives moving between
// matches and tabs. The backend relays it over a WebSocket (handlers/irc.rs) and keeps one
// connection per user, shared by all of their browser tabs: a tab opened later joins it without
// logging in. It's for talking: the site's bots run and log the rooms, and every page reads a
// room's chat from them. A tab says which match it has open (`attend`), and the room's bot then
// makes the referee one of the room's referees and this connection joins the room's channel, so
// what they type there comes from their own account. Private messages are kept here.

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

/** A private conversation, under the other person's nick. */
export interface Conversation {
  name: string;
  lines: ChatLine[];
}

type ServerMsg =
  | { type: "connected"; nick: string; session: number }
  | { type: "login_failed" }
  | { type: "login_blocked"; retry_after: number }
  | { type: "no_session" }
  | { type: "bot_account" }
  | { type: "message"; from: string; target: string; text: string; action: boolean; seq?: number }
  | {
      type: "past_message";
      from: string;
      target: string;
      text: string;
      action: boolean;
      at: number;
      seq: number;
    }
  | { type: "joined"; channel: string }
  | { type: "parted"; channel: string }
  | { type: "conversation_closed"; name: string }
  | { type: "error"; message: string }
  | { type: "closed"; reason: CloseReason };

interface IrcApi {
  status: IrcStatus;
  /** Our nick once logged in. */
  nick: string | null;
  /** Why the last connection failed or ended, or what Bancho last refused. */
  error: string | null;
  /** Private conversations, keyed by conversationKey(). */
  conversations: Record<string, Conversation>;
  /** The room channels the connection is in, by conversationKey(). */
  channels: ReadonlySet<string>;
  /** Logs in, or joins the connection another tab already has. */
  connect: (username: string, password: string) => void;
  /** Ends the connection for every tab. */
  disconnect: () => void;
  send: (target: string, text: string) => void;
  /** Closes a private conversation in every tab; a new message starts it again. */
  close: (name: string) => void;
  /**
   * The match this tab has open, or none: its room's bot makes the referee one of the room's
   * referees and the connection joins the room's channel. Again whenever the match's rooms change.
   */
  attend: (matchId: string | null) => void;
}

const MAX_LINES = 2000;

export const conversationKey = (name: string) => name.toLowerCase();
export const isChannel = (name: string) => name.startsWith("#");

const IrcContext = createContext<IrcApi | null>(null);

// Tells this browser's other tabs that we're online, so the ones that aren't join in.
const TABS_CHANNEL = "bancho-irc";

/** Why the relay closed: `CloseReason` in backend/src/bancho/relay.rs. */
type CloseReason =
  | "no_session"
  | "invalid_login"
  | "bot_account"
  | "account_check_failed"
  | "too_many_failed_logins"
  | "login_expected"
  | "no_longer_referee"
  | "fell_behind"
  | "bancho_unreachable"
  | "login_timed_out"
  | "login_failed"
  | "bancho_closed"
  | "invalid_line"
  | "every_tab_closed"
  | "quit_in_another_tab";

export const CLOSE_REASON_TEXT: Record<CloseReason, string> = {
  no_session: "sürdürülecek bağlantı yok",
  invalid_login: "giriş bilgileri geçersiz",
  bot_account: "bu hesap sitenin botlarından biri",
  account_check_failed: "hesap kontrol edilemedi",
  too_many_failed_logins: "çok fazla hatalı giriş denemesi",
  login_expected: "giriş bekleniyordu",
  no_longer_referee: "artık hakem değilsin",
  fell_behind: "bu sekme geride kaldı",
  bancho_unreachable: "Bancho'ya ulaşılamadı",
  login_timed_out: "Bancho girişe cevap vermedi",
  login_failed: "giriş başarısız",
  bancho_closed: "Bancho bağlantıyı kapattı",
  invalid_line: "Bancho geçersiz bir satır gönderdi",
  every_tab_closed: "bütün sekmeler kapandı",
  quit_in_another_tab: "başka bir sekmede bağlantı kesildi",
};

export function IrcProvider({ children }: { children: ReactNode }) {
  const canReferee = hasRole(useSignedInUser(), "referee");
  const [status, setStatus] = useState<IrcStatus>("offline");
  const [nick, setNick] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [conversations, setConversations] = useState<Record<string, Conversation>>({});
  const [channels, setChannels] = useState<ReadonlySet<string>>(new Set());
  const socket = useRef<WebSocket | null>(null);
  const nickRef = useRef<string | null>(null);
  const nextId = useRef(1);
  // Private messages from before this tab joined (the backend keeps them for the connection) get
  // negative ids, like saved room lines, so they aren't unread.
  const nextPastId = useRef(-1);
  // The connection this tab last joined and the last private message it has from it, so joining
  // it again doesn't repeat what the tab already shows.
  const seen = useRef({ session: 0, seq: 0 });
  const tabs = useRef<BroadcastChannel | null>(null);
  // The match this tab has open, said again whenever the connection is (re)made.
  const attending = useRef<string | null>(null);

  const sendJson = useCallback((msg: object) => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify(msg));
  }, []);

  const drop = useCallback((name: string) => {
    setConversations((all) => {
      const rest = { ...all };
      delete rest[conversationKey(name)];
      return rest;
    });
  }, []);

  const addLine = useCallback(
    (name: string, line: Omit<ChatLine, "id" | "at">, past?: { at: number }) => {
      const full = past
        ? { ...line, id: nextPastId.current--, at: past.at }
        : { ...line, id: nextId.current++, at: Date.now() };
      setConversations((all) => {
        const key = conversationKey(name);
        const current = all[key] ?? { name, lines: [] };
        return { ...all, [key]: { ...current, lines: [...current.lines, full].slice(-MAX_LINES) } };
      });
    },
    [],
  );

  const setJoined = useCallback((channel: string, joined: boolean) => {
    setChannels((all) => {
      const key = conversationKey(channel);
      if (all.has(key) === joined) return all;
      const next = new Set(all);
      if (joined) next.add(key);
      else next.delete(key);
      return next;
    });
  }, []);

  // A private message lives under the other person's name, whoever sent it.
  const conversationOf = useCallback(({ from, target }: { from: string; target: string }) => {
    const fromMe = from.toLowerCase() === nickRef.current?.toLowerCase();
    return fromMe ? target : from;
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
          if (attending.current) sendJson({ type: "attend", match_id: attending.current });
          break;
        case "login_failed":
          setError("Bancho girişi reddetti: kullanıcı adını ve IRC şifresini kontrol et.");
          break;
        case "bot_account":
          setError(
            "Bu hesap sitenin botlarından biri: onunla bağlanırsan bot Bancho'dan düşer. Hostlar odaya bot üzerinden yazar.",
          );
          break;
        case "login_blocked":
          setError(
            `Çok fazla hatalı giriş denemesi. ${Math.ceil(msg.retry_after / 60)} dakika sonra tekrar dene.`,
          );
          break;
        case "message":
          if (msg.seq) seen.current.seq = msg.seq;
          addLine(conversationOf(msg), {
            from: msg.from,
            text: msg.text,
            action: msg.action,
            notice: false,
          });
          break;
        case "past_message":
          if (msg.seq <= seen.current.seq) break;
          seen.current.seq = msg.seq;
          addLine(
            conversationOf(msg),
            { from: msg.from, text: msg.text, action: msg.action, notice: false },
            msg,
          );
          break;
        case "joined":
          setJoined(msg.channel, true);
          break;
        case "parted":
          setJoined(msg.channel, false);
          break;
        case "conversation_closed":
          drop(msg.name);
          break;
        case "error": {
          // "Someone: No such nick" goes into that conversation; a room's refusal is shown above the chat.
          const subject = msg.message.split(":")[0];
          if (isChannel(subject)) setError(`Bancho: ${msg.message}`);
          else addLine(subject, { from: "Bancho", text: msg.message, action: false, notice: true });
          break;
        }
        case "closed":
          setError(
            (e) => e ?? `Bağlantı kapandı: ${CLOSE_REASON_TEXT[msg.reason] ?? "bilinmeyen sebep"}`,
          );
          break;
      }
    },
    [addLine, conversationOf, drop, sendJson, setJoined],
  );

  // Closes this tab's socket; the connection lives on while other tabs use it.
  const leave = useCallback(() => {
    const ws = socket.current;
    socket.current = null;
    ws?.close();
    nickRef.current = null;
    setNick(null);
    setStatus("offline");
    setChannels(new Set());
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
    sendJson({ type: "quit" });
    leave();
  }, [leave, sendJson]);

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

  const send = useCallback(
    (target: string, text: string) => sendJson({ type: "send", target, text }),
    [sendJson],
  );
  const close = useCallback(
    (name: string) => {
      drop(name);
      sendJson({ type: "close", name });
    },
    [drop, sendJson],
  );
  const attend = useCallback(
    (matchId: string | null) => {
      attending.current = matchId;
      sendJson({ type: "attend", match_id: matchId });
    },
    [sendJson],
  );

  const api = useMemo<IrcApi>(
    () => ({
      status,
      nick,
      error,
      conversations,
      channels,
      connect,
      disconnect,
      send,
      close,
      attend,
    }),
    [status, nick, error, conversations, channels, connect, disconnect, send, close, attend],
  );

  return <IrcContext.Provider value={api}>{children}</IrcContext.Provider>;
}

export function useIrc(): IrcApi {
  const api = useContext(IrcContext);
  if (!api) throw new Error("useIrc needs an IrcProvider");
  return api;
}
