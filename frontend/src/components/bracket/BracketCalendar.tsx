import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
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

/** Pixels per hour on the day view's timeline. */
const HOUR_H = 56;
/** How long a match is drawn on the timeline, in minutes; they have no end time. */
const MATCH_MINUTES = 60;

/** Local "YYYY-MM-DD", the key matches are grouped by. */
function dayKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

function minutesIntoDay(d: Date): number {
  return d.getHours() * 60 + d.getMinutes();
}

// Opens on the month of the next match still to be played, or of the last one.
function initialMonth(matches: BracketMatch[]): Date {
  const now = new Date().toISOString();
  const scheduled = matches.flatMap((m) => (m.starts_at ? [m.starts_at] : [])).sort();
  const pick = scheduled.find((at) => at >= now) ?? scheduled[scheduled.length - 1];
  const d = pick ? new Date(pick) : new Date();
  return new Date(d.getFullYear(), d.getMonth(), 1);
}

/** The current time, updated every 30 seconds. */
function useNow(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return now;
}

interface Placed {
  match: BracketMatch;
  start: number;
  column: number;
  columns: number;
}

// Side by side columns for matches whose hours overlap, like a calendar app: each
// group of overlapping matches splits the width between as many columns as it needs.
function layoutDay(matches: BracketMatch[]): Placed[] {
  const placed: Placed[] = [];
  let group: Placed[] = [];
  let columnEnds: number[] = [];
  let groupEnd = -Infinity;
  const closeGroup = () => {
    for (const p of group) p.columns = columnEnds.length;
    group = [];
    columnEnds = [];
  };
  for (const match of matches) {
    const start = minutesIntoDay(new Date(match.starts_at!));
    if (start >= groupEnd) closeGroup();
    let column = columnEnds.findIndex((end) => end <= start);
    if (column === -1) column = columnEnds.length;
    columnEnds[column] = start + MATCH_MINUTES;
    groupEnd = Math.max(groupEnd, start + MATCH_MINUTES);
    const p = { match, start, column, columns: 1 };
    group.push(p);
    placed.push(p);
  }
  closeGroup();
  return placed;
}

/** A month at a time, Monday first, or one day's timeline; plus the matches that have
    no time yet. */
export function BracketCalendar({ matches, teams, myTeamId, viewerId, selectedId, onSelect }: Props) {
  const [month, setMonth] = useState(() => initialMonth(matches));
  // The day shown on its own, or null for the month.
  const [day, setDay] = useState<Date | null>(null);
  const now = useNow();
  const today = dayKey(now);

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

  const isMine = (m: BracketMatch) =>
    (viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId)) ||
    (myTeamId != null && (m.team1_id === myTeamId || m.team2_id === myTeamId));

  const shiftMonth = (by: number) => setMonth((m) => new Date(m.getFullYear(), m.getMonth() + by, 1));
  const shiftDay = (by: number) => setDay((d) => d && new Date(d.getFullYear(), d.getMonth(), d.getDate() + by));
  const backToMonth = () => {
    if (day) setMonth(new Date(day.getFullYear(), day.getMonth(), 1));
    setDay(null);
  };
  const goToday = () =>
    day ? setDay(startOfDay(new Date())) : setMonth(new Date(new Date().getFullYear(), new Date().getMonth(), 1));

  function chip(m: BracketMatch, style?: CSSProperties) {
    const names = [m.team1_id, m.team2_id].map((id) => (id && teams.get(id)?.name) || "?").join(" – ");
    const classes = [
      "cal-match",
      `is-${m.side}`,
      m.concluded_at && "is-done",
      isMine(m) && "is-mine",
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
      <button key={m.id} type="button" className={classes} style={style} onClick={() => onSelect(m.id)}>
        {content}
      </button>
    ) : (
      <div key={m.id} className={classes} style={style}>
        {content}
      </div>
    );
  }

  return (
    <div className="bracket-calendar">
      <div className="cal-toolbar">
        {day ? (
          <>
            <IconButton icon="chevronLeft" label="Önceki gün" variant="outlined" onClick={() => shiftDay(-1)} />
            <h2 className="cal-title">
              {day.toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric", weekday: "long" })}
            </h2>
            <IconButton icon="chevronRight" label="Sonraki gün" variant="outlined" onClick={() => shiftDay(1)} />
          </>
        ) : (
          <>
            <IconButton icon="chevronLeft" label="Önceki ay" variant="outlined" onClick={() => shiftMonth(-1)} />
            <h2 className="cal-title">{month.toLocaleDateString("tr-TR", { month: "long", year: "numeric" })}</h2>
            <IconButton icon="chevronRight" label="Sonraki ay" variant="outlined" onClick={() => shiftMonth(1)} />
          </>
        )}
        <div className="cal-toolbar-end">
          {day && (
            <button type="button" className="btn btn-secondary btn-sm" onClick={backToMonth}>
              Aya dön
            </button>
          )}
          <button type="button" className="btn btn-secondary btn-sm" onClick={goToday}>
            Bugün
          </button>
        </div>
      </div>

      {day ? (
        <DayTimeline
          day={day}
          now={now}
          placed={layoutDay(byDay.get(dayKey(day)) ?? [])}
          chip={chip}
        />
      ) : (
        <MonthGrid month={month} today={today} byDay={byDay} isMine={isMine} onOpenDay={setDay} />
      )}

      {unscheduled.length > 0 && (
        <div className="cal-unscheduled">
          <h3>Zamanı belirlenmemiş maçlar</h3>
          <div className="cal-unscheduled-list">{unscheduled.map((m) => chip(m))}</div>
        </div>
      )}
    </div>
  );
}

interface MonthGridProps {
  month: Date;
  today: string;
  byDay: Map<string, BracketMatch[]>;
  isMine: (m: BracketMatch) => boolean;
  onOpenDay: (day: Date) => void;
}

/** Six weeks (or five) with each day's match count; a day opens its timeline. */
function MonthGrid({ month, today, byDay, isMine, onOpenDay }: MonthGridProps) {
  // Six weeks from the Monday on or before the 1st.
  const start = new Date(month);
  start.setDate(1 - ((month.getDay() + 6) % 7));
  const days = Array.from({ length: 42 }, (_, i) => new Date(start.getFullYear(), start.getMonth(), start.getDate() + i));
  // Drop a trailing week that's entirely in the next month.
  const shown = days[35].getMonth() !== month.getMonth() ? days.slice(0, 35) : days;

  return (
    <div className="cal-frame">
      <div className="cal-grid">
        {WEEKDAYS.map((d) => (
          <div key={d} className="cal-weekday">
            {d}
          </div>
        ))}
        {shown.map((d) => {
          const key = dayKey(d);
          const dayMatches = byDay.get(key) ?? [];
          const count = dayMatches.length;
          const classes = [
            "cal-day",
            d.getMonth() !== month.getMonth() && "is-outside",
            key === today && "is-today",
            count > 0 && "has-matches",
          ]
            .filter(Boolean)
            .join(" ");
          const date = d.toLocaleDateString("tr-TR", { day: "numeric", month: "long", weekday: "long" });
          return (
            <button
              key={key}
              type="button"
              className={classes}
              aria-label={count > 0 ? `${date}, ${count} maç` : `${date}, maç yok`}
              onClick={() => onOpenDay(d)}
            >
              <span className="cal-day-number">{d.getDate()}</span>
              {count > 0 && (
                <span className={dayMatches.some(isMine) ? "cal-day-count is-mine" : "cal-day-count"}>
                  {count}
                  <span className="cal-day-count-label"> maç</span>
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

interface DayTimelineProps {
  day: Date;
  now: Date;
  placed: Placed[];
  chip: (m: BracketMatch, style?: CSSProperties) => ReactNode;
}

/** The day's 24 hours, matches placed at their start time, and a line at the current
    time when it's today. */
function DayTimeline({ day, now, placed, chip }: DayTimelineProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isToday = dayKey(day) === dayKey(now);
  const nowTop = (minutesIntoDay(now) / 60) * HOUR_H;

  // On opening a day, scroll to the current time on today, else to the first match,
  // with an hour above it for context.
  const key = dayKey(day);
  const firstStart = placed[0]?.start;
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const minutes = isToday ? minutesIntoDay(new Date()) : (firstStart ?? 8 * 60);
    el.scrollTop = Math.max(0, ((minutes - 60) / 60) * HOUR_H);
    // Keyed on the day alone so the clock ticking doesn't scroll it back.
  }, [key]);

  return (
    <div className="cal-frame">
      <div className="cal-day-summary">{placed.length > 0 ? `${placed.length} maç` : "Bu gün maç yok"}</div>
      <div className="cal-timeline-scroll" ref={scrollRef}>
        <div className="cal-timeline" style={{ height: 24 * HOUR_H }}>
          {Array.from({ length: 24 }, (_, h) => (
            <div key={h} className="cal-hour" style={{ top: h * HOUR_H, height: HOUR_H }}>
              <span className="cal-hour-label">{`${String(h).padStart(2, "0")}:00`}</span>
            </div>
          ))}
          <div className="cal-timeline-matches">
            {placed.map((p) =>
              chip(p.match, {
                top: (p.start / 60) * HOUR_H + 1,
                // A match late in the evening stops at midnight.
                height: (Math.min(MATCH_MINUTES, 24 * 60 - p.start) / 60) * HOUR_H - 3,
                left: `calc(${(p.column / p.columns) * 100}% + 2px)`,
                width: `calc(${100 / p.columns}% - 4px)`,
              }),
            )}
          </div>
          {isToday && (
            <div className="cal-now" style={{ top: nowTop }} role="presentation">
              <span className="cal-now-label">{formatClock(now.toISOString())}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
