import { Fragment, useEffect, useLayoutEffect, useRef, useState, type FormEvent } from "react";
import { IconButton } from "../../../components/IconButton";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import { BANCHO_BOT, normalizeName, type SlotTeam } from "../../../lib/bancho";
import type { RoomChatView } from "./useRefereeRoom";
import "./ChatPanel.css";

interface Props {
  /** The match's rooms, oldest first. */
  rooms: RoomChatView[];
  /** Players' team colours by normalizeName(username), to colour their names. */
  colors: Map<string, SlotTeam | null>;
}

const URL_RE = /(https?:\/\/[^\s]+)/g;

function Linkified({ text }: { text: string }) {
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
 * Bancho chat, IRC style: a tab per room of the match (what was saved, then what's
 * live) and the private conversations. Other matches' rooms stay out of it, even
 * ones the connection is still in.
 */
export function ChatPanel({ rooms, colors }: Props) {
  const irc = useIrc();
  const latest = rooms.length > 0 ? rooms[rooms.length - 1].channel : null;
  const [active, setActive] = useState<string | null>(latest);
  const [draft, setDraft] = useState("");
  // The last line seen in each conversation, for the unread counts.
  const [seen, setSeen] = useState<Record<string, number>>({});
  // Room tabs the referee closed. One comes back if we're in its channel again.
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const log = useRef<HTMLDivElement>(null);
  const stuckToBottom = useRef(true);

  useEffect(() => setActive(latest), [latest]);

  const roomTabs = rooms.filter((r) => r.joined || !hidden.has(conversationKey(r.channel)));
  const dmKeys = Object.keys(irc.conversations)
    .filter((k) => !isChannel(k))
    .sort((a, b) => (a === "banchobot" ? -1 : b === "banchobot" ? 1 : a.localeCompare(b)));

  // A closed tab falls back to the latest room.
  const keyOf = (name: string | null) => (name ? conversationKey(name) : null);
  const tabKeys = [...roomTabs.map((r) => conversationKey(r.channel)), ...dmKeys];
  const activeKey = tabKeys.includes(keyOf(active) ?? "") ? keyOf(active) : (tabKeys[0] ?? null);
  const activeRoom = roomTabs.find((r) => conversationKey(r.channel) === activeKey);
  const activeDm = activeRoom ? undefined : activeKey ? irc.conversations[activeKey] : undefined;
  const lines = activeRoom?.lines ?? activeDm?.lines ?? [];
  // Only live lines count as unread; saved ones have negative ids (see roomChat.ts).
  const lastLiveId = lines.reduce((max, l) => Math.max(max, l.id), 0);

  // Viewing a conversation marks it read.
  useEffect(() => {
    if (activeKey) setSeen((s) => (s[activeKey] === lastLiveId ? s : { ...s, [activeKey]: lastLiveId }));
  }, [activeKey, lastLiveId]);

  // Follows new lines unless the referee has scrolled up to read.
  useLayoutEffect(() => {
    const el = log.current;
    if (el && stuckToBottom.current) el.scrollTop = el.scrollHeight;
  }, [lines.length, activeKey]);

  const online = irc.status === "online";
  // A room we're not in only shows what was saved.
  const target = activeRoom ? (activeRoom.joined ? activeRoom.channel : null) : (activeDm?.name ?? null);
  const unread = (key: string, tabLines: ChatLine[]) => tabLines.filter((l) => l.id > (seen[key] ?? 0)).length;

  function submit(e: FormEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text || !online) return;
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

  function closeRoom(channel: string) {
    setHidden((h) => new Set(h).add(conversationKey(channel)));
    irc.close(channel);
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

  const placeholder = activeRoom && !activeRoom.joined
    ? "Bu odada değilsin: kayıtlı sohbet salt okunur"
    : !online
      ? "Bağlı değilsin"
      : target
        ? `${target} — mesaj veya !mp komutu`
        : "/msg kişi mesaj";

  return (
    <section className="chat-panel" aria-label="Bancho sohbeti">
      <div className="chat-tabs" role="tablist">
        {tabKeys.length === 0 && <span className="chat-tabs-empty muted">Henüz sohbet yok</span>}
        {roomTabs.map((r) => {
          const key = conversationKey(r.channel);
          const label = rooms.length > 1 ? `${rooms.indexOf(r) + 1}. oda` : r.channel;
          // A room's tab closes once we've left its channel.
          return tab(key, label, r.channel, unread(key, r.lines), r.joined ? null : () => closeRoom(r.channel), r.channel);
        })}
        {dmKeys.map((key) => {
          const c = irc.conversations[key];
          return tab(key, c.name, c.name, unread(key, c.lines), () => irc.close(c.name), c.name);
        })}
      </div>

      {activeRoom && !activeRoom.joined && (
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
            {activeRoom ? "Bu odada henüz mesaj yok." : online ? "Mesaj yok." : "Sohbet için Bancho'ya bağlan."}
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
          disabled={!online}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button type="submit" className="btn btn-primary" disabled={!online || !draft.trim()}>
          Gönder
        </button>
      </form>
    </section>
  );
}

function ChatRow({ line, me, colors }: { line: ChatLine; me: string | null; colors: Map<string, SlotTeam | null> }) {
  const fromMe = me !== null && line.from.toLowerCase() === me.toLowerCase();
  const color = colors.get(normalizeName(line.from));
  const classes = [
    "chat-line",
    line.from === BANCHO_BOT && "is-bot",
    fromMe && "is-me",
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
