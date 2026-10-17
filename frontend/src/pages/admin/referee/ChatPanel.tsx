import { Fragment, useEffect, useLayoutEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { IconButton } from "../../../components/IconButton";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import { BANCHO_BOT, normalizeName, type SlotTeam } from "../../../lib/bancho";
import type { RoomChatView } from "./useRefereeRoom";
import "./ChatPanel.css";

interface Props {
  /** The match's rooms, oldest first, each with what its bot saved. */
  rooms: RoomChatView[];
  /** Players' team colours by normalizeName(username), to colour their names. */
  colors: Map<string, SlotTeam | null>;
  /** Shown above the tabs: the referee's Bancho login, or who's connected. */
  top?: ReactNode;
  /** False while the panel is behind another tab on a phone: lines stay unread until it's back. */
  visible?: boolean;
  /** Told the unread lines across every conversation, for the phone's tab bar. */
  onUnread?: (count: number) => void;
  /**
   * Hosts write in the match's current room through its bot (their accounts are the bots, which
   * can't join each other's rooms): its name, why it can't take a line right now, and the send.
   */
  viaBot?: { bot: string | null; blocker: string | null; send: (text: string) => Promise<boolean> };
}

const URL_RE = /(https?:\/\/[^\s]+)/g;

export function Linkified({ text }: { text: string }) {
  const parts = text.split(URL_RE);
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <a key={i} href={part} target="_blank" rel="noopener noreferrer">
            {part}
          </a>
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
}

const time = (at: number) =>
  new Date(at).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });

/**
 * The match's chat, IRC style: a tab per room of the match, each what its bot saved and saves as it
 * comes (the same for everyone on the match, whether or not they're on Bancho), and the referee's
 * private conversations. What the referee types in a room goes out from their own account, once
 * the room's bot has let them into its channel.
 */
export function ChatPanel({ rooms, colors, top, visible = true, onUnread, viaBot }: Props) {
  const irc = useIrc();
  const latest = rooms.length > 0 ? rooms[rooms.length - 1].channel : null;
  const [active, setActive] = useState<string | null>(latest);
  const [draft, setDraft] = useState("");
  // The last line seen in each conversation, for the unread counts.
  const [seen, setSeen] = useState<Record<string, number>>({});
  const log = useRef<HTMLDivElement>(null);
  const stuckToBottom = useRef(true);

  useEffect(() => setActive(latest), [latest]);

  const dmKeys = Object.keys(irc.conversations).sort((a, b) =>
    a === "banchobot" ? -1 : b === "banchobot" ? 1 : a.localeCompare(b),
  );

  // A closed tab falls back to the latest room.
  const keyOf = (name: string | null) => (name ? conversationKey(name) : null);
  const tabKeys = [...rooms.map((r) => conversationKey(r.channel)), ...dmKeys];
  const activeKey = tabKeys.includes(keyOf(active) ?? "") ? keyOf(active) : (tabKeys[0] ?? null);
  const activeRoom = rooms.find((r) => conversationKey(r.channel) === activeKey);
  const activeDm = activeRoom ? undefined : activeKey ? irc.conversations[activeKey] : undefined;
  const lines = activeRoom?.lines ?? activeDm?.lines ?? [];
  // Only lines that arrived while the page was open count as unread; what was loaded with it has
  // negative ids (see roomChat.ts).
  const lastId = lines.reduce((max, l) => Math.max(max, l.id), 0);

  // Viewing a conversation marks it read.
  useEffect(() => {
    if (visible && activeKey) setSeen((s) => (s[activeKey] === lastId ? s : { ...s, [activeKey]: lastId }));
  }, [visible, activeKey, lastId]);

  // Follows new lines unless the referee has scrolled up to read. A hidden log can't scroll, so
  // coming back into view catches up.
  useLayoutEffect(() => {
    const el = log.current;
    if (visible && el && stuckToBottom.current) el.scrollTop = el.scrollHeight;
  }, [visible, lines.length, activeKey]);

  const online = irc.status === "online";
  // A host's line goes to the current room through its bot.
  const throughBot = viaBot !== undefined && activeRoom?.current === true;
  // A room takes what's typed once the referee is in its channel (or, through the bot, once the
  // bot is in it); a private conversation while online.
  const target = activeRoom
    ? throughBot
      ? viaBot.blocker === null
        ? activeRoom.channel
        : null
      : online && activeRoom.joined
        ? activeRoom.channel
        : null
    : online
      ? (activeDm?.name ?? null)
      : null;
  const [sending, setSending] = useState(false);
  const unread = (key: string, tabLines: ChatLine[]) => tabLines.filter((l) => l.id > (seen[key] ?? 0)).length;
  const totalUnread =
    rooms.reduce((n, r) => n + unread(conversationKey(r.channel), r.lines), 0) +
    dmKeys.reduce((n, key) => n + unread(key, irc.conversations[key].lines), 0);
  useEffect(() => onUnread?.(totalUnread), [onUnread, totalUnread]);

  function submit(e: FormEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    if (throughBot) {
      if (!target || sending) return;
      setSending(true);
      viaBot
        .send(text)
        .then((sent) => {
          if (sent) setDraft("");
        })
        .finally(() => setSending(false));
      stuckToBottom.current = true;
      return;
    }
    if (!online) return;
    const [command, ...rest] = text.split(" ");
    if (command.toLowerCase() === "/msg" || command.toLowerCase() === "/query") {
      if (!rest[0] || isChannel(rest[0])) return;
      if (rest.length > 1) irc.send(rest[0], rest.slice(1).join(" "));
      setActive(rest[0]);
    } else if (target) {
      irc.send(target, text);
    } else {
      return;
    }
    setDraft("");
    stuckToBottom.current = true;
  }

  const tab = (key: string, label: string, title: string, badge: number, onClose: (() => void) | null, name: string) => (
    <span key={key} className={`chat-tab ${key === activeKey ? "is-active" : ""}`}>
      <button
        type="button"
        role="tab"
        aria-selected={key === activeKey}
        className="chat-tab-button"
        title={title}
        onClick={() => setActive(name)}
      >
        {label}
        {badge > 0 && key !== activeKey && <span className="chat-tab-unread">{badge}</span>}
      </button>
      {onClose && (
        <IconButton icon="close" iconSize={12} className="chat-tab-close" label={`${title} sohbetini kapat`} onClick={onClose} />
      )}
    </span>
  );

  let placeholder: string;
  if (throughBot) {
    placeholder = viaBot.blocker ?? `${viaBot.bot ?? "Bot"} üzerinden yaz`;
  } else if (activeRoom) {
    placeholder = !activeRoom.current
      ? "Eski oda: kayıtlı sohbet, salt okunur"
      : !online
        ? "Yazmak için Bancho'ya bağlan"
        : !activeRoom.joined
          ? "Bot seni odanın hakemi yapıyor…"
          : `${activeRoom.channel}: mesaj veya !mp komutu`;
  } else {
    placeholder = !online ? "Bağlı değilsin" : target ? `${target}: mesaj` : "/msg kişi mesaj";
  }

  return (
    <section className="chat-panel" aria-label="Bancho sohbeti">
      {top}
      <div className="chat-tabs" role="tablist">
        {tabKeys.length === 0 && <span className="chat-tabs-empty muted">Henüz sohbet yok</span>}
        {rooms.map((r) => {
          const key = conversationKey(r.channel);
          const label = rooms.length > 1 ? `${rooms.indexOf(r) + 1}. oda` : r.channel;
          return tab(key, label, r.channel, unread(key, r.lines), null, r.channel);
        })}
        {dmKeys.map((key) => {
          const c = irc.conversations[key];
          return tab(key, c.name, c.name, unread(key, c.lines), () => irc.close(c.name), c.name);
        })}
      </div>

      {activeRoom && !activeRoom.current && (
        <p className="chat-archive-note">
          {activeRoom.channel} · kayıtlı sohbet · {lines.length} mesaj
        </p>
      )}

      <div
        className="chat-log"
        ref={log}
        onScroll={(e) => {
          const el = e.currentTarget;
          stuckToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {lines.length === 0 && (
          <p className="chat-empty muted">
            {activeRoom ? "Bu odada henüz mesaj yok." : online ? "Mesaj yok." : "Özel mesajlar için Bancho'ya bağlan."}
          </p>
        )}
        {lines.map((line) => (
          <ChatRow key={line.id} line={line} me={irc.nick} colors={colors} />
        ))}
      </div>

      <form className="chat-input" onSubmit={submit}>
        <input
          className="input"
          aria-label={target ? `${target} sohbetine yaz` : "Mesaj"}
          placeholder={placeholder}
          disabled={throughBot ? !target : !online || (activeRoom !== undefined && !target)}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button
          type="submit"
          className="btn btn-primary"
          disabled={
            !draft.trim() || sending || (throughBot ? !target : !online || (activeRoom !== undefined && !target))
          }
        >
          Gönder
        </button>
      </form>
    </section>
  );
}

interface ChatRowProps {
  line: ChatLine;
  /** Our nick, whose lines stand out. */
  me: string | null;
  colors: Map<string, SlotTeam | null>;
  /** The match's referee, whose lines stand out on the public page. */
  referee?: string | null;
}

/** One chat line, IRC style: the time, who said it in their team's colour, what they said. */
export function ChatRow({ line, me, colors, referee }: ChatRowProps) {
  const fromMe = me !== null && line.from.toLowerCase() === me.toLowerCase();
  const fromReferee = referee != null && normalizeName(line.from) === normalizeName(referee);
  const color = colors.get(normalizeName(line.from));
  const classes = [
    "chat-line",
    line.from === BANCHO_BOT && "is-bot",
    fromMe && "is-me",
    fromReferee && "is-referee",
    line.notice && "is-notice",
    color && `is-team-${color}`,
    colors.has(normalizeName(line.from)) && "is-player",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes}>
      <time className="chat-time">{time(line.at)}</time>
      <span className="chat-from">{line.action ? `* ${line.from}` : line.from}</span>
      <span className="chat-text">
        <Linkified text={line.text} />
      </span>
    </div>
  );
}
