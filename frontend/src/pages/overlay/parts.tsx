import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { API_URL } from "../../api/client";
import type { PublicMatch, Team } from "../../api/types";
import { firstTo } from "../../lib/bestOf";
import type { PoolEntry } from "../../lib/matchPool";
import type { Tally } from "../../lib/liveRoom";
import "./Overlay.css";

// The pieces both overlay pages are drawn from, after the designers' tournament-client designs.

/** Team 1 (blue on the room's slots, green on stream) or team 2 (red, purple). */
export type Side = 1 | 2;

export const SIDE_CLASS: Record<Side, string> = { 1: "is-side-1", 2: "is-side-2" };

/**
 * The 1920×1080 frame an OBS browser source shows at its own size, scaled down to fit anything
 * smaller (a preview in a browser). `holes` are left transparent for the tournament client
 * underneath: rectangles in frame pixels, with the corners each one rounds.
 */
export function OverlayStage({
  className,
  holes = [],
  children,
}: {
  className: string;
  holes?: Hole[];
  children: ReactNode;
}) {
  const [scale, setScale] = useState(1);
  useLayoutEffect(() => {
    const fit = () => setScale(Math.min(window.innerWidth / 1920, window.innerHeight / 1080));
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, []);
  const ground =
    holes.length > 0
      ? { clipPath: `path(evenodd, "M0 0H1920V1080H0Z ${holes.map(holePath).join(" ")}")` }
      : undefined;
  return (
    <div className="ov-viewport">
      <div className={`ov-stage ${className}`} style={{ "--ov-scale": scale } as CSSProperties}>
        <div className="ov-ground" style={ground} aria-hidden="true" />
        {children}
      </div>
    </div>
  );
}

export interface Hole {
  x: number;
  y: number;
  w: number;
  h: number;
  /** Radius of each corner, clockwise from the top left. */
  r: [number, number, number, number];
}

function holePath({ x, y, w, h, r: [tl, tr, br, bl] }: Hole): string {
  return [
    `M${x + tl} ${y}`,
    `H${x + w - tr}`,
    tr ? `A${tr} ${tr} 0 0 1 ${x + w} ${y + tr}` : "",
    `V${y + h - br}`,
    br ? `A${br} ${br} 0 0 1 ${x + w - br} ${y + h}` : "",
    `H${x + bl}`,
    bl ? `A${bl} ${bl} 0 0 1 ${x} ${y + h - bl}` : "",
    `V${y + tl}`,
    tl ? `A${tl} ${tl} 0 0 1 ${x + tl} ${y}` : "",
    "Z",
  ].join("");
}

export type CrystalKind = "won" | "open" | "pick" | "ban";

/**
 * The tournament's crystal, the one mark the overlays speak in: a filled crystal is a map won (a
 * star pip) or a pick, a hollow one a win still to come, a struck-through one a ban. A pick's
 * outline is the picking team's colour and its heart the colour of the team that won the map (the
 * picker's until it's played).
 */
export function Crystal({ kind, className = "" }: { kind: CrystalKind; className?: string }) {
  return (
    <svg
      className={`ov-crystal is-${kind} ${className}`}
      viewBox="-13 -13 26 26"
      aria-hidden="true"
      focusable="false"
    >
      <path
        className="ov-crystal-star"
        d="M0 -11C1 -3.2 3.2 -1 11 0C3.2 1 1 3.2 0 11C-1 3.2 -3.2 1 -11 0C-3.2 -1 -1 -3.2 0 -11Z"
      />
      {kind !== "open" && kind !== "ban" && (
        <path
          className="ov-crystal-heart"
          d="M0 -6C.5 -1.6 1.6 -.5 6 0C1.6 .5 .5 1.6 0 6C-.5 1.6 -1.6 .5 -6 0C-1.6 -.5 -.5 -1.6 0 -6Z"
        />
      )}
      {kind === "ban" && <path className="ov-crystal-strike" d="M-12.5 4.2L12.5 -4.2" />}
    </svg>
  );
}

/** A team's banner in its pill; without one, the team's initial. */
export function TeamBanner({ team, side }: { team: Team | null; side: Side }) {
  return (
    <div className={`ov-banner ${SIDE_CLASS[side]}`}>
      {team?.banner_url ? (
        <img src={`${API_URL}${team.banner_url}`} alt="" />
      ) : (
        <span className="ov-banner-initial">
          {team?.name.trim().charAt(0).toLocaleUpperCase("tr-TR") ?? ""}
        </span>
      )}
    </div>
  );
}

/**
 * A team's map wins as crystals: filled for each map won, hollow for the ones still needed, filled
 * from the frame's edge inwards (team 2's row is mirrored). Without a best of, just the wins.
 */
export function StarPips({
  wins,
  bestOf,
  side,
}: {
  wins: number;
  bestOf: number | null;
  side: Side;
}) {
  const total = bestOf ? Math.max(firstTo(bestOf), wins) : wins;
  const pips = Array.from({ length: total }, (_, i) => i < wins);
  return (
    <div
      className={`ov-pips ${SIDE_CLASS[side]}`}
      role="img"
      aria-label={`${wins} galibiyet`}
      style={{ "--pip-count": total } as CSSProperties}
    >
      {pips.map((won, i) =>
        // A pip remounts when it's won, so it lights up as the result is revealed.
        won ? (
          <Crystal key={`won-${i}`} kind="won" className="ov-pip is-lit" />
        ) : (
          <Crystal key={`open-${i}`} kind="open" className="ov-pip" />
        ),
      )}
    </div>
  );
}

/** The stage's name over the middle of the frame ("Grand Final"). */
export function StageTitle({ match }: { match: PublicMatch | null }) {
  return <h1 className="ov-stage-title">{match?.stage?.name ?? ""}</h1>;
}

/** The banner's trophy in a frost glow that takes both teams' colours, at the foot of the frame. */
export function TrophyGlow({ className }: { className: string }) {
  return (
    <div className={`ov-trophy ${className}`} aria-hidden="true">
      <span className="ov-trophy-gears" />
      <span className="ov-trophy-mark" />
    </div>
  );
}

/** "#rrggbb" → the CSS variables a map's category colours a pill with. */
export function categoryStyle(entry: PoolEntry): CSSProperties {
  return { "--ov-cat": entry.color } as CSSProperties;
}

/** The category's short name: "NM" for NM3. */
export function categoryName(entry: PoolEntry): string {
  return entry.abbreviation;
}

/** Every pool map's state on stream: who banned or picked it, and who won it. */
export interface MapMark {
  kind: "ban" | "pick";
  /** The side that banned or picked it. */
  side: Side;
  /** Who won it once it's played (a pick), else null. */
  winner: Side | null;
  /** When the referee marked it, for ordering. */
  at: string;
}

/** The bans and picks of a bracket match by pool map, with each played pick's winner. */
export function mapMarks(match: PublicMatch, tally: Tally | null): Map<string, MapMark> {
  const sideOf = (teamId: string | null): Side | null =>
    teamId === match.team1_id ? 1 : teamId === match.team2_id ? 2 : null;
  const marks = new Map<string, MapMark>();
  for (const ban of match.bans) {
    const side = sideOf(ban.team_id);
    if (side) marks.set(ban.map_id, { kind: "ban", side, winner: null, at: ban.created_at });
  }
  for (const pick of match.picks) {
    const side = sideOf(pick.team_id);
    if (side) marks.set(pick.map_id, { kind: "pick", side, winner: null, at: pick.created_at });
  }
  // The latest counted result of each map decides it.
  for (const row of tally?.rows ?? []) {
    if (!row.counted || !row.mapId) continue;
    const winner = sideOf(row.winnerId);
    const mark = marks.get(row.mapId);
    if (mark && winner) mark.winner = winner;
    else if (!mark && winner) marks.set(row.mapId, { kind: "pick", side: winner, winner, at: "" });
  }
  return marks;
}

/** A side's crystal for a marked map. */
export function MarkCrystal({ mark }: { mark: MapMark }) {
  if (mark.kind === "ban")
    return <Crystal kind="ban" className={`ov-mark ${SIDE_CLASS[mark.side]}`} />;
  const heart = mark.winner ?? mark.side;
  return <Crystal kind="pick" className={`ov-mark ${SIDE_CLASS[mark.side]} heart-${heart}`} />;
}

/** "Artist - Title [Version]", as the pills scroll it. */
export function mapLine(entry: PoolEntry): string {
  const m = entry.map;
  return `${m.artist} - ${m.title} [${m.version}]`;
}

/**
 * Text that scrolls sideways when it's wider than its box, and otherwise sits still: centred, or at
 * the start with `start`.
 */
export function Marquee({
  text,
  className,
  start = false,
}: {
  text: string;
  className: string;
  start?: boolean;
}) {
  const box = useRef<HTMLSpanElement>(null);
  const copy = useRef<HTMLSpanElement>(null);
  const [overflow, setOverflow] = useState(false);
  useLayoutEffect(() => {
    const measure = () => {
      if (box.current && copy.current)
        setOverflow(copy.current.scrollWidth > box.current.clientWidth + 1);
    };
    measure();
    // The display and condensed faces load after the first layout.
    document.fonts?.ready.then(measure).catch(() => {});
  }, [text]);
  return (
    <span
      ref={box}
      className={`ov-marquee ${start ? "is-start" : ""} ${overflow ? "is-scrolling" : ""} ${className}`}
    >
      <span
        className="ov-marquee-line"
        style={{ "--ov-marquee-s": Math.max(8, text.length * 0.32) } as CSSProperties}
      >
        <span ref={copy} className="ov-marquee-copy">
          {text}
        </span>
        {overflow && (
          <span className="ov-marquee-copy" aria-hidden="true">
            {text}
          </span>
        )}
      </span>
    </span>
  );
}

/**
 * A pool map as a pill: its cover, its category's tag over the top, the crystal of its ban or pick
 * and the side's tint washing in from the left (crimson for a ban), and, with `title`, the map
 * scrolling along its foot in the category's colour. The map on in the room gets the frost ring.
 */
export function MapPill({
  entry,
  mark,
  tag,
  current,
  title,
}: {
  entry: PoolEntry;
  mark: MapMark | undefined;
  tag: string;
  current: boolean;
  title: boolean;
}) {
  const state = mark
    ? mark.kind === "ban"
      ? "is-banned"
      : `is-picked ${SIDE_CLASS[mark.side]}`
    : "is-open";
  return (
    <div className={`ov-pill ${state} ${current ? "is-current" : ""}`} style={categoryStyle(entry)}>
      <div className="ov-pill-cover">
        {entry.map.cover_url && <img src={entry.map.cover_url} alt="" />}
        <span className="ov-pill-tint" />
      </div>
      {mark && <MarkCrystal mark={mark} />}
      <span className="ov-pill-tag">{tag}</span>
      {title && <Marquee className="ov-pill-title" text={mapLine(entry)} />}
    </div>
  );
}

/**
 * A page's title over the middle of the frame, for the pages that aren't one match's: the title in
 * the display face with a wide light line under it.
 */
export function Masthead({ title, line }: { title: string; line?: string | null }) {
  return (
    <header className="ov-masthead">
      <h1>{title}</h1>
      {line && <p>{line}</p>}
    </header>
  );
}

/** A team's banner as a small pill, for rows and cards; its initial without one. */
export function BannerChip({
  team,
  className = "",
}: {
  team: Team | null | undefined;
  className?: string;
}) {
  return (
    <span className={`ov-banner-chip ${className}`}>
      {team?.banner_url ? (
        <img src={`${API_URL}${team.banner_url}`} alt="" />
      ) : (
        <span>{team?.name.trim().charAt(0).toLocaleUpperCase("tr-TR") ?? ""}</span>
      )}
    </span>
  );
}
