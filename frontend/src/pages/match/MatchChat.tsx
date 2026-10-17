import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { isRoomNoise, type SlotTeam } from "../../lib/bancho";
import { readJson, writeJson } from "../../lib/storage";
import { ChatRow } from "../admin/referee/ChatPanel";
import type { PublicRoomChat } from "./useMatchPage";
import "../admin/referee/ChatPanel.css";
import "./MatchChat.css";

interface Props {
  /** The match's rooms, oldest first. */
  rooms: PublicRoomChat[];
  /** Players' team colours by normalizeName(username). */
  colors: Map<string, SlotTeam | null>;
  /** The match's referee, whose lines stand out. */
  referee: string | null;
  /** The saved chat has arrived: lines from then on are new. */
  loaded: boolean;
  /** The match is live, so its chat is streamed. */
  live: boolean;
  /** The live socket is up, so new lines arrive as they're said. */
  connected: boolean;
  /** False while the panel is behind another tab on a phone: new lines stay unread. */
  visible: boolean;
  /** Told how many lines arrived while the panel wasn't looked at, for the phone's tab bar. */
  onUnread: (count: number) => void;
}

const FILTER_KEY = "match-chat-filtered";

/**
 * The room's chat as it happened, read only: a tab per room when one had to be replaced, the
 * saved lines followed by the live ones. "Sadeleştir" (on by default, remembered per browser)
 * hides the room being run, BanchoBot's housekeeping and the referee's !mp commands, and keeps
 * what the players and referee say and the match itself: maps set, started, finished and scored.
 */
export function MatchChat({ rooms, colors, referee, loaded, live, connected, visible, onUnread }: Props) {
  const latest = rooms.length > 0 ? rooms[rooms.length - 1].mpId : null;
  const [active, setActive] = useState<number | null>(latest);
  useEffect(() => setActive(latest), [latest]);
  const room = rooms.find((r) => r.mpId === active) ?? rooms[rooms.length - 1];

  const [filtered, setFiltered] = useState(() => readJson<boolean>(FILTER_KEY, true));
  useEffect(() => writeJson(FILTER_KEY, filtered), [filtered]);

  const all = room?.lines ?? [];
  const lines = useMemo(() => (filtered ? all.filter((l) => !isRoomNoise(l.from, l.text)) : all), [all, filtered]);
  const hidden = all.length - lines.length;

  // Lines that arrived while the panel was out of sight; what was there when the page loaded is read.
  const shown = lines.length;
  const [seen, setSeen] = useState<number | null>(null);
  useEffect(() => {
    if (loaded && (visible || seen === null)) setSeen(shown);
  }, [loaded, visible, shown, seen]);
  const unread = seen === null ? 0 : Math.max(0, shown - seen);
  useEffect(() => onUnread(unread), [onUnread, unread]);

  // Follows new lines unless the reader has scrolled up.
  const log = useRef<HTMLDivElement>(null);
  const stuckToBottom = useRef(true);
  useLayoutEffect(() => {
    const el = log.current;
    if (visible && el && stuckToBottom.current) el.scrollTop = el.scrollHeight;
  }, [visible, shown, active, filtered]);

  return (
    <section className="chat-panel match-chat" aria-label="Oda sohbeti">
      <header className="match-chat-head">
        <h2 className="match-chat-title">Oda sohbeti</h2>
        <label
          className="match-chat-filter"
          title="BanchoBot'un oda kurulum mesajlarını (girişler, slotlar, ayarlar, sayaçlar) ve hakemin !mp komutlarını gizler"
        >
          <input type="checkbox" role="switch" checked={filtered} onChange={(e) => setFiltered(e.target.checked)} />
          <span>Sadeleştir</span>
          {filtered && hidden > 0 && <span className="match-chat-hidden num">{hidden} gizli</span>}
        </label>
      </header>

      {rooms.length > 1 && (
        <div className="chat-tabs" role="tablist" aria-label="Odalar">
          {rooms.map((r, i) => (
            <span key={r.mpId} className={`chat-tab ${r.mpId === room?.mpId ? "is-active" : ""}`}>
              <button
                type="button"
                role="tab"
                aria-selected={r.mpId === room?.mpId}
                className="chat-tab-button"
                title={`#mp_${r.mpId}`}
                onClick={() => setActive(r.mpId)}
              >
                {i + 1}. oda{r.current ? "" : " (eski)"}
              </button>
            </span>
          ))}
        </div>
      )}

      <div
        className="chat-log"
        ref={log}
        role="log"
        aria-label="Mesajlar"
        onScroll={(e) => {
          const el = e.currentTarget;
          stuckToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {!room && <p className="chat-empty muted">Oda açılınca sohbeti burada canlı akar.</p>}
        {room && lines.length === 0 && (
          <p className="chat-empty muted">
            {all.length > 0 ? "Henüz maçla ilgili bir mesaj yok. Hepsini görmek için Sadeleştir'i kapat." : "Bu odada henüz mesaj yok."}
          </p>
        )}
        {lines.map((line) => (
          <ChatRow key={line.id} line={line} me={null} colors={colors} referee={referee} />
        ))}
      </div>

      {live ? (
        <footer className={`match-chat-foot${connected ? " is-live" : ""}`} role="status">
          {connected ? "Canlı: yeni mesajlar geldikçe akar" : "Bağlantı koptu, yeniden bağlanıyor…"}
        </footer>
      ) : (
        room && <footer className="match-chat-foot is-saved">Maç bitti: kayıtlı sohbet</footer>
      )}
    </section>
  );
}
