import { useEffect, useState } from "react";
import type { RoomState } from "../../lib/bancho";
import { formatClock, formatLength } from "../../lib/format";
import type { PoolEntry } from "../../lib/matchPool";
import "./MapClock.css";

/** The current time, ticking while `active`. */
export function useNow(active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, [active]);
  return now;
}

/**
 * The map the room has on and, while it's played, a countdown of its length from BanchoBot's
 * "The match has started!" (as the bot saw it), so the referee knows when to expect the result. The length is the
 * pool's, already adjusted for the map's mods (DT, HT). A map outside the pool has no known
 * length, so its clock counts up instead. Under the name, the room's mods and team mode.
 */
export function MapClock({
  lobby,
  currentEntry,
  mapStartedAt,
  emptyText,
}: {
  lobby: RoomState;
  /** The pool entry the room has on, if it's a pool map. */
  currentEntry: PoolEntry | undefined;
  /** When the map being played started, ms. */
  mapStartedAt: number | null;
  /** What to say while the room has no map. */
  emptyText: string;
}) {
  const startedAt = lobby.playing ? mapStartedAt : null;
  const playing = startedAt !== null;
  const now = useNow(playing);
  const roomInfo = [lobby.mods || "Mod yok", lobby.teamMode, lobby.winCondition]
    .filter(Boolean)
    .join(" · ");
  if (lobby.beatmapId === null) {
    return (
      <div className="lobby-map is-empty">
        <span className="lobby-map-empty">{emptyText}</span>
        {lobby.teamMode && <span className="lobby-map-meta">{roomInfo}</span>}
      </div>
    );
  }

  const length = currentEntry?.map.total_length ?? null;
  const elapsed = startedAt !== null ? Math.max(0, Math.floor((now - startedAt) / 1000)) : 0;
  const remaining = length === null ? null : length - elapsed;
  const over = playing && remaining !== null && remaining < 0;
  const endsAt =
    startedAt !== null && length !== null ? formatClock(startedAt + length * 1000) : null;

  let figure: string;
  let caption: string;
  if (!playing) {
    figure = length === null ? "—" : formatLength(length);
    caption = length === null ? "havuzda değil" : "uzunluk";
  } else if (remaining === null) {
    figure = formatLength(elapsed);
    caption = "geçti · uzunluğu bilinmiyor";
  } else if (over) {
    figure = `+${formatLength(-remaining)}`;
    caption = "süre doldu, sonuç bekleniyor";
  } else {
    figure = formatLength(remaining);
    caption = `kaldı · ${endsAt} gibi biter`;
  }
  const progress = playing && length ? Math.min(1, elapsed / length) : 0;

  return (
    <div className={`lobby-map${playing ? " is-playing" : ""}${over ? " is-over" : ""}`}>
      <div className="lobby-map-text">
        <span className="lobby-map-title">
          {currentEntry && (
            <span
              className="lobby-map-label"
              style={{ color: currentEntry.color, borderColor: currentEntry.color }}
            >
              {currentEntry.label}
            </span>
          )}
          <a
            className="lobby-map-name"
            href={`https://osu.ppy.sh/b/${lobby.beatmapId}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {lobby.beatmapName ?? lobby.beatmapId}
          </a>
        </span>
        <span className="lobby-map-meta">{roomInfo}</span>
      </div>
      <div
        className="lobby-map-clock"
        role="timer"
        aria-label={playing ? "Map süresi" : "Map uzunluğu"}
      >
        <span className="lobby-map-figure">{figure}</span>
        <span className="lobby-map-caption">{caption}</span>
      </div>
      {playing && length !== null && (
        <span className="lobby-map-progress" aria-hidden="true">
          <span style={{ transform: `scaleX(${progress})` }} />
        </span>
      )}
    </div>
  );
}
