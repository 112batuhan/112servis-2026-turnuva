import { useEffect, useState, type FormEvent } from "react";
import type { AutorefLogEntry } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import type { SlotTeam } from "../../../lib/bancho";
import { formatClockSeconds } from "../../../lib/format";
import { useFollowLog } from "../../../lib/useFollowLog";
import type { ChatLine } from "../../../lib/roomChat";
import type { RoomChatView } from "./useRefereeRoom";
import { ChatRow } from "../../../components/match/ChatRow";
import "./ChatPanel.css";

/** The log tab's key among the rooms' channels. */
const LOG_TAB = "autoref";

const LEVEL_LABELS: Record<AutorefLogEntry["level"], string> = {
  info: "bilgi",
  warn: "uyarı",
  error: "hata",
};

interface Props {
  /** The match's rooms, oldest first, each with what its bot saved. */
  rooms: RoomChatView[];
  /** Players' team colours by normalizeName(username), to colour their names. */
  colors: Map<string, SlotTeam | null>;
  /** False while the panel is behind another tab on a phone: lines stay unread until it's back. */
  visible?: boolean;
  /** Told the unread lines across every room, for the phone's tab bar. */
  onUnread?: (count: number) => void;
  /**
   * Writing in the match's current room, which goes out through its bot: the bot's name, why it
   * can't take a line right now, and the send (resolves to whether it went out).
   */
  say: {
    bot: string | null;
    blocker: string | null;
    /** The auto referee runs the match: messages go out, !mp commands wait for manual mode. */
    auto: boolean;
    send: (text: string) => Promise<boolean>;
  };
  /** What the auto referee did and why, in a tab of its own after the rooms. */
  log: AutorefLogEntry[];
}

/**
 * The match's chat, IRC style: a tab per room of the match, each what its bot saved and saves as it
 * comes (the same for everyone on the match), then the auto referee's log. What the referee types
 * in the current room goes out through its bot, signed with their name; older rooms and the log
 * are read-only.
 */
export function ChatPanel({ rooms, colors, visible = true, onUnread, say, log }: Props) {
  const user = useSignedInUser();
  const latest = rooms.length > 0 ? rooms[rooms.length - 1].channel : null;
  const [active, setActive] = useState<string | null>(latest);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  // The last line seen in each room, for the unread counts.
  const [seen, setSeen] = useState<Record<string, number>>({});
  // The log's warnings and errors count until it's looked at (opening the page counts as looking).
  const [logSeenAt, setLogSeenAt] = useState(() => Date.now());

  useEffect(() => setActive(latest), [latest]);

  const showingLog = active === LOG_TAB;
  const activeRoom = showingLog
    ? undefined
    : (rooms.find((r) => r.channel === active) ?? rooms[rooms.length - 1]);
  const activeKey = showingLog ? LOG_TAB : (activeRoom?.channel ?? null);
  const lines = activeRoom?.lines ?? [];
  // Only lines that arrived while the page was open count as unread; what was loaded with it has
  // negative ids (see roomChat.ts).
  const lastId = lines.reduce((max, l) => Math.max(max, l.id), 0);

  // Viewing a room marks it read.
  useEffect(() => {
    if (visible && activeKey && !showingLog)
      setSeen((s) => (s[activeKey] === lastId ? s : { ...s, [activeKey]: lastId }));
  }, [visible, activeKey, lastId, showingLog]);
  useEffect(() => {
    if (visible && showingLog) setLogSeenAt(Date.now());
  }, [visible, showingLog, log.length]);
  const logAlerts = log.filter(
    (l) => l.level !== "info" && new Date(l.created_at).getTime() > logSeenAt,
  ).length;

  // Follows new lines unless the referee has scrolled up to read.
  const follow = useFollowLog(visible, `${activeKey}:${lines.length}:${log.length}`);

  const unread = (key: string, tabLines: ChatLine[]) =>
    tabLines.filter((l) => l.id > (seen[key] ?? 0)).length;
  const totalUnread = rooms.reduce((n, r) => n + unread(r.channel, r.lines), 0);
  useEffect(() => onUnread?.(totalUnread), [onUnread, totalUnread]);

  // Only the current room takes a line, once its bot is in it.
  const writable = !showingLog && activeRoom?.current === true && say.blocker === null;

  function submit(e: FormEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text || !writable || sending) return;
    setSending(true);
    say
      .send(text)
      .then((sent) => {
        if (sent) setDraft("");
      })
      .finally(() => setSending(false));
    follow.toEnd();
  }

  let placeholder: string;
  if (showingLog) placeholder = "Otomatik hakemin kaydı: salt okunur";
  else if (!activeRoom) placeholder = "Önce oda aç";
  else if (!activeRoom.current) placeholder = "Eski oda: kayıtlı sohbet, salt okunur";
  else if (say.blocker) placeholder = say.blocker;
  else if (say.auto)
    placeholder = `${say.bot ?? "Bot"} üzerinden yaz: mesaj (!mp manuel kontrolde)`;
  else placeholder = `${say.bot ?? "Bot"} üzerinden yaz: mesaj veya !mp komutu`;

  return (
    <section className="chat-panel" aria-label="Bancho sohbeti">
      <div className="chat-tabs" role="tablist">
        {rooms.length === 0 && <span className="chat-tabs-empty muted">Henüz sohbet yok</span>}
        {rooms.map((r, i) => {
          const isActive = r.channel === activeKey;
          const badge = unread(r.channel, r.lines);
          return (
            <span key={r.channel} className={`chat-tab ${isActive ? "is-active" : ""}`}>
              <button
                type="button"
                role="tab"
                aria-selected={isActive}
                className="chat-tab-button"
                title={r.channel}
                onClick={() => setActive(r.channel)}
              >
                {rooms.length > 1 ? `${i + 1}. oda` : r.channel}
                {badge > 0 && !isActive && <span className="chat-tab-unread">{badge}</span>}
              </button>
            </span>
          );
        })}
        <span className={`chat-tab chat-tab-log ${showingLog ? "is-active" : ""}`}>
          <button
            type="button"
            role="tab"
            aria-selected={showingLog}
            className="chat-tab-button"
            title="Otomatik hakemin ne yaptığı ve neden: hata ayıklamak için"
            onClick={() => setActive(LOG_TAB)}
          >
            Otomatik hakem
            {logAlerts > 0 && !showingLog && (
              <span className="chat-tab-unread is-alert">
                {logAlerts}
                <span className="sr-only"> yeni uyarı</span>
              </span>
            )}
          </button>
        </span>
      </div>

      {activeRoom && !activeRoom.current && (
        <p className="chat-archive-note">
          {activeRoom.channel} · kayıtlı sohbet · {lines.length} mesaj
        </p>
      )}

      <div className="chat-log" ref={follow.ref} onScroll={follow.onScroll}>
        {showingLog ? (
          log.length === 0 ? (
            <p className="chat-empty muted">
              Otomatik hakem bu maçta henüz bir şey yapmadı. Oda açınca, oyuncuları beklerken ve her
              adımda buraya yazar.
            </p>
          ) : (
            log.map((l) => (
              <div key={l.id} className={`chat-line autoref-line is-${l.level}`}>
                <time className="chat-time">{formatClockSeconds(l.created_at)}</time>
                <span className="chat-from">{LEVEL_LABELS[l.level]}</span>
                <span className="chat-text">{l.message}</span>
              </div>
            ))
          )
        ) : (
          <>
            {lines.length === 0 && (
              <p className="chat-empty muted">
                {activeRoom ? "Bu odada henüz mesaj yok." : "Oda açılınca sohbeti burada görünür."}
              </p>
            )}
            {lines.map((line) => (
              <ChatRow key={line.id} line={line} me={user.username} colors={colors} />
            ))}
          </>
        )}
      </div>

      <form className="chat-input" onSubmit={submit}>
        <input
          className="input"
          aria-label={activeRoom ? `${activeRoom.channel} sohbetine yaz` : "Mesaj"}
          placeholder={placeholder}
          disabled={!writable}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button
          type="submit"
          className="btn btn-primary"
          disabled={!draft.trim() || sending || !writable}
        >
          Gönder
        </button>
      </form>
    </section>
  );
}
