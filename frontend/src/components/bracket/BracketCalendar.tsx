import { useMemo, useState } from "react";
import type { BracketMatch, Team } from "../../api/types";
import { compareByCode, compareByTime, matchCode } from "../../lib/bracket";
import { formatClock } from "../../lib/format";
import { IconButton } from "../IconButton";
import "./BracketCalendar.css";

interface Props {
  matches: BracketMatch[];
  teams: Map<string, Team>;
  myTeamId?: string | null;
  viewerId?: string;
  selectedId?: string | null;
  onSelect?: (id: string) => void;
}

const WEEKDAYS = ["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"];

/** Local "YYYY-MM-DD", the key matches are grouped by. */
function dayKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

// Opens on the month of the next match still to be played, or of the last one.
function initialMonth(matches: BracketMatch[]): Date {
  const now = new Date().toISOString();
  const scheduled = matches.flatMap((m) => (m.starts_at ? [m.starts_at] : [])).sort();
  const pick = scheduled.find((at) => at >= now) ?? scheduled[scheduled.length - 1];
  const d = pick ? new Date(pick) : new Date();
  return new Date(d.getFullYear(), d.getMonth(), 1);
}

/** A month at a time, Monday first, plus the matches that have no time yet. */
export function BracketCalendar({ matches, teams, myTeamId, viewerId, selectedId, onSelect }: Props) {
  const [month, setMonth] = useState(() => initialMonth(matches));

  const byDay = useMemo(() => {
    const map = new Map<string, BracketMatch[]>();
    for (const m of [...matches].sort(compareByTime)) {
      if (!m.starts_at) continue;
      const key = dayKey(new Date(m.starts_at));
      map.set(key, [...(map.get(key) ?? []), m]);
    }
    return map;
  }, [matches]);
  const unscheduled = matches.filter((m) => !m.starts_at).sort(compareByCode);

  // Six weeks from the Monday on or before the 1st.
  const start = new Date(month);
  start.setDate(1 - ((month.getDay() + 6) % 7));
  const days = Array.from({ length: 42 }, (_, i) => new Date(start.getFullYear(), start.getMonth(), start.getDate() + i));
  // Drop a trailing week that's entirely in the next month.
  const shown = days[35].getMonth() !== month.getMonth() ? days.slice(0, 35) : days;
  const today = dayKey(new Date());

  const shift = (by: number) => setMonth((m) => new Date(m.getFullYear(), m.getMonth() + by, 1));

  function chip(m: BracketMatch) {
    const names = [m.team1_id, m.team2_id].map((id) => (id && teams.get(id)?.name) || "?").join(" – ");
    const mine =
      (viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId)) ||
      (myTeamId != null && (m.team1_id === myTeamId || m.team2_id === myTeamId));
    const classes = [
      "cal-match",
      `is-${m.side}`,
      m.concluded_at && "is-done",
      mine && "is-mine",
      m.id === selectedId && "is-selected",
    ]
      .filter(Boolean)
      .join(" ");
    const content = (
      <>
        <span className="cal-match-head">
          {m.starts_at && <span className="cal-match-time">{formatClock(m.starts_at)}</span>}
          <strong>{matchCode(m)}</strong>
        </span>
        <span className="cal-match-teams">{names}</span>
      </>
    );
    return onSelect ? (
      <button key={m.id} type="button" className={classes} onClick={() => onSelect(m.id)}>
        {content}
      </button>
    ) : (
      <div key={m.id} className={classes}>
        {content}
      </div>
    );
  }

  return (
    <div className="bracket-calendar">
      <div className="cal-toolbar">
        <IconButton icon="chevronLeft" label="Önceki ay" variant="outlined" onClick={() => shift(-1)} />
        <h2 className="cal-month">{month.toLocaleDateString("tr-TR", { month: "long", year: "numeric" })}</h2>
        <IconButton icon="chevronRight" label="Sonraki ay" variant="outlined" onClick={() => shift(1)} />
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={() => setMonth(new Date(new Date().getFullYear(), new Date().getMonth(), 1))}
        >
          Bugün
        </button>
      </div>

      <div className="cal-scroll">
        <div className="cal-grid">
          {WEEKDAYS.map((d) => (
            <div key={d} className="cal-weekday">
              {d}
            </div>
          ))}
          {shown.map((d) => {
            const key = dayKey(d);
            const classes = [
              "cal-day",
              d.getMonth() !== month.getMonth() && "is-outside",
              key === today && "is-today",
            ]
              .filter(Boolean)
              .join(" ");
            return (
              <div key={key} className={classes}>
                <span className="cal-day-number">{d.getDate()}</span>
                {byDay.get(key)?.map(chip)}
              </div>
            );
          })}
        </div>
      </div>

      {unscheduled.length > 0 && (
        <div className="cal-unscheduled">
          <h3>Zamanı belirlenmemiş maçlar</h3>
          <div className="cal-unscheduled-list">{unscheduled.map(chip)}</div>
        </div>
      )}
    </div>
  );
}
