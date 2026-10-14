import { Fragment, useEffect, useLayoutEffect, useRef, useState, type FormEvent } from "react";
import { IconButton } from "../../../components/IconButton";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import { BANCHO_BOT, normalizeName, type SlotTeam } from "../../../lib/bancho";
import "./ChatPanel.css";

interface Props {
  /** The match's room channel, opened by default. */
  channel: string | null;
  /** A concluded match's saved room chat, shown read-only in place of the live channel. */
  archive: ChatLine[] | null;
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
 * Bancho chat, IRC style: the match's room channel and private conversations. Other
 * rooms' channels stay out of it, even ones the connection is still in.
 */
export function ChatPanel({ channel, archive, colors }: Props) {
  const irc = useIrc();
  const [active, setActive] = useState<string | null>(channel);
  const [draft, setDraft] = useState("");
  // The last line seen in each conversation, for the unread counts.
  const [seen, setSeen] = useState<Record<string, number>>({});
  const log = useRef<HTMLDivElement>(null);
  const stuckToBottom = useRef(true);

  useEffect(() => setActive(channel), [channel]);

  const roomKey = channel ? conversationKey(channel) : null;
  const room = roomKey ? irc.conversations[roomKey] : undefined;
  const dmKeys = Object.keys(irc.conversations)
    .filter((k) => !isChannel(k))
    .sort((a, b) => (a === "banchobot" ? -1 : b === "banchobot" ? 1 : a.localeCompare(b)));
  const showRoom = channel !== null && (archive !== null || room !== undefined);

  // A closed conversation falls back to the room.
  const activeKey = active && (conversationKey(active) === roomKey || dmKeys.includes(conversationKey(active)))
    ? conversationKey(active)
    : roomKey;
  const viewingArchive = activeKey === roomKey && archive !== null;
  const current = activeKey ? irc.conversations[activeKey] : undefined;
  const lines = (viewingArchive ? archive : current?.lines) ?? [];
  const lastId = lines.length > 0 ? lines[lines.length - 1].id : 0;

  // Viewing a conversation marks it read.
  useEffect(() => {
    if (activeKey && !viewingArchive) setSeen((s) => (s[activeKey] === lastId ? s : { ...s, [activeKey]: lastId }));
  }, [activeKey, lastId, viewingArchive]);

  // Follows new lines unless the referee has scrolled up to read.
  useLayoutEffect(() => {
    const el = log.current;
    if (el && stuckToBottom.current) el.scrollTop = el.scrollHeight;
  }, [lastId, activeKey]);

  const online = irc.status === "online";
  const target = viewingArchive ? null : (current?.name ?? (activeKey === roomKey ? channel : null));
  const unread = (key: string) => irc.conversations[key]?.lines.filter((l) => l.id > (seen[key] ?? 0)).length ?? 0;

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
    }
    setDraft("");
    stuckToBottom.current = true;
  }

  const tab = (key: string, name: string, badge: number, closable: boolean) => (
    <span key={key} className={`chat-tab ${key === activeKey ? "is-active" : ""}`}>
      <button
        type="button"
        role="tab"
        aria-selected={key === activeKey}
        className="chat-tab-button"
        onClick={() => setActive(name)}
      >
        {name}
        {badge > 0 && key !== activeKey && <span className="chat-tab-unread">{badge}</span>}
      </button>
      {closable && (
        <IconButton
          icon="close"
          iconSize={12}
          className="chat-tab-close"
          label={`${name} sohbetini kapat`}
          onClick={() => irc.close(name)}
        />
      )}
    </span>
  );

  const placeholder = viewingArchive
    ? "Maç bitti: kayıtlı sohbet salt okunur"
    : !online
      ? "Bağlı değilsin"
      : target
        ? `${target} — mesaj veya !mp komutu`
        : "/msg kişi mesaj";

  return (
    <section className="chat-panel" aria-label="Bancho sohbeti">
      <div className="chat-tabs" role="tablist">
        {!showRoom && dmKeys.length === 0 && <span className="chat-tabs-empty muted">Henüz sohbet yok</span>}
        {showRoom &&
          roomKey &&
          // A live room's tab closes once we've left its channel.
          tab(roomKey, channel!, viewingArchive ? 0 : unread(roomKey), archive === null && room !== undefined && !room.joined)}
        {dmKeys.map((key) => tab(key, irc.conversations[key].name, unread(key), true))}
      </div>

      {viewingArchive && <p className="chat-archive-note">Bu odanın kayıtlı sohbeti · {lines.length} mesaj</p>}

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
            {viewingArchive
              ? "Bu oda için kayıtlı sohbet yok."
              : online
                ? "Mesaj yok."
                : "Sohbet için Bancho'ya bağlan."}
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
          disabled={!online || viewingArchive}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button type="submit" className="btn btn-primary" disabled={!online || viewingArchive || !draft.trim()}>
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
