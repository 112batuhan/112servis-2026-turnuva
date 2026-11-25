import { useState, type FormEvent } from "react";
import { Icon } from "../../../../components/Icon";
import { ABORT_WINDOW_MS, REPORT_MAX } from "../../../../lib/playerRoom";
import type { MatchPageState } from "../../../../lib/useMatchPage";
import type { RoomMe } from "./PlayerRoomPage";
import "./RoomActions.css";

interface Props {
  page: MatchPageState;
  me: RoomMe;
  now: number;
  busy: boolean;
  onAbort: () => Promise<boolean>;
  onReport: (text: string) => Promise<boolean>;
  onStop: () => Promise<boolean>;
}

type Open = "report" | "stop" | null;

/**
 * What a player can reach for at any moment: abort a map in its first 30 seconds (on the team's
 * account), report something to the hosts and referees, and the emergency stop that hands the match
 * to a referee. The same as !abort, !rapor and !panik in the room, which keep working there.
 */
export function RoomActions({ page, me, now, busy, onAbort, onReport, onStop }: Props) {
  const [open, setOpen] = useState<Open>(null);
  const [text, setText] = useState("");
  const [done, setDone] = useState<string | null>(null);
  const { match, mapStartedAt } = page;
  if (!match) return null;

  const autoref = page.liveRoom?.autoref ?? null;
  const hasRoom = match.rooms.length > 0 && !match.concluded_at;
  const aborts = autoref?.teams.find((t) => t.team_id === me.teamId)?.aborts_left ?? null;
  const windowLeft =
    autoref?.phase === "playing" && mapStartedAt !== null
      ? mapStartedAt + ABORT_WINDOW_MS - now
      : null;
  const canAbort =
    autoref?.enabled === true && windowLeft !== null && windowLeft > 0 && (aborts ?? 0) > 0;
  const abortNote = !autoref?.enabled
    ? "Otomatik hakem açıkken"
    : aborts === 0
      ? "Hakkınız kalmadı"
      : windowLeft !== null && windowLeft > 0
        ? `${Math.ceil(windowLeft / 1000)} sn içinde`
        : "İlk 30 sn";

  const say = (message: string) => {
    setDone(message);
    setOpen(null);
  };

  const sendReport = async (e: FormEvent) => {
    e.preventDefault();
    if (!text.trim()) return;
    if (await onReport(text.trim())) {
      setText("");
      say("Raporun hostlara ve hakemlere iletildi.");
    }
  };

  return (
    <section className="room-actions" aria-label="Maç kontrolleri">
      <div className="room-actions-row">
        <button
          type="button"
          className="room-action is-abort"
          disabled={!canAbort || busy}
          onClick={async () => {
            if (await onAbort()) say("Map iptal edildi, aynı map tekrar açılacak.");
          }}
        >
          <span className="room-action-name">
            <Icon name="undo" size={15} />
            Abort
          </span>
          <span className="room-action-note">
            {aborts !== null && (
              <>
                <span className="num">{aborts}</span> hak ·{" "}
              </>
            )}
            {abortNote}
          </span>
        </button>
        <button
          type="button"
          className="room-action"
          aria-expanded={open === "report"}
          disabled={!hasRoom}
          onClick={() => {
            setDone(null);
            setOpen(open === "report" ? null : "report");
          }}
        >
          <span className="room-action-name">
            <Icon name="send" size={15} />
            Rapor
          </span>
          <span className="room-action-note">Hostlara, hakemlere</span>
        </button>
        <button
          type="button"
          className="room-action is-stop"
          aria-expanded={open === "stop"}
          disabled={!hasRoom}
          onClick={() => {
            setDone(null);
            setOpen(open === "stop" ? null : "stop");
          }}
        >
          <span className="room-action-name">
            <Icon name="stop" size={15} />
            Durdur
          </span>
          <span className="room-action-note">Acil durumda</span>
        </button>
      </div>

      {open === "report" && (
        <form className="room-actions-panel" onSubmit={sendReport}>
          <label className="room-actions-label" htmlFor="room-report">
            Ne oldu? Hostlar ve hakemler bildirim ve Discord'dan görür.
          </label>
          <textarea
            id="room-report"
            className="input room-actions-text"
            value={text}
            maxLength={REPORT_MAX}
            rows={3}
            onChange={(e) => setText(e.target.value)}
            autoFocus
          />
          <div className="room-actions-buttons">
            <span className="room-actions-count num">
              {text.length}/{REPORT_MAX}
            </span>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={() => setOpen(null)}
            >
              Vazgeç
            </button>
            <button
              type="submit"
              className="btn btn-primary btn-sm"
              disabled={busy || !text.trim()}
            >
              Gönder
            </button>
          </div>
        </form>
      )}

      {open === "stop" && (
        <div className="room-actions-panel is-stop" role="group" aria-label="Durdur">
          <p className="room-actions-warning">
            Otomatik hakem durur ve maç bir hakeme devredilir; hostlar hemen bilgilendirilir. Sadece
            acil durumlarda kullan, kötüye kullanım raporlanır.
          </p>
          <div className="room-actions-buttons">
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={() => setOpen(null)}
            >
              Vazgeç
            </button>
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={async () => {
                if (await onStop()) say("Maç durduruldu, hostlar bilgilendirildi.");
              }}
            >
              <Icon name="stop" size={14} />
              Maçı durdur
            </button>
          </div>
        </div>
      )}

      {done && (
        <p className="room-actions-done" role="status">
          <Icon name="check" size={14} />
          {done}
        </p>
      )}
    </section>
  );
}
