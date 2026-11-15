import { useEffect, useMemo, useRef, type CSSProperties, type RefObject } from "react";
import type { BracketMatch, PublicBracket, Team } from "../../api/types";
import { isLive, matchCode, scoreHalves, winningSlot } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import {
  boardBounds,
  CARD_H,
  CARD_W,
  clampCamera,
  fitCamera,
  UNIT,
  VIEW,
  type Camera,
} from "./bracketBoard";
import { BannerChip, Masthead, OverlayStage, SIDE_CLASS, type Side } from "./parts";
import "./BracketPage.css";

// The bracket on stream: the editor's board drawn in the overlay's language, looked at through a
// camera the dock moves (pan, zoom, fit, focus). The camera glides to where the dock points with a
// critically damped spring, written straight to the board's transform each frame, so a held arrow
// in the dock reads as one smooth move rather than a series of steps.

// Card layout in board pixels: the code and time over the two team rows.
const HEAD_H = 26;
const ROW_H = (CARD_H - HEAD_H) / 2;

export function BracketPage({
  bracket,
  camera,
  matchId,
}: {
  bracket: PublicBracket | null;
  camera: Camera | null;
  matchId: string | null;
}) {
  const matches = useMemo(() => bracket?.matches ?? [], [bracket]);
  const bounds = useMemo(() => boardBounds(matches), [matches]);
  const target = camera ? clampCamera(camera, bounds) : fitCamera(bounds);
  const board = useRef<HTMLDivElement>(null);
  useSmoothCamera(board, target, matches.length > 0);
  const teams = bracket?.teams ?? [];
  const byId = useMemo(() => new Map(matches.map((m) => [m.id, m])), [matches]);

  return (
    <OverlayStage className="br">
      <Masthead title="Bracket" line={null} />
      <div className="br-view">
        <div ref={board} className="br-board">
          <svg className="br-links" aria-hidden="true">
            {matches.flatMap((m) =>
              (["winner", "loser"] as const).flatMap((outcome) => {
                const next = outcome === "winner" ? m.winner_to : m.loser_to;
                const to = next ? byId.get(next.match_id) : undefined;
                if (!next || !to) return [];
                return [
                  <path
                    key={`${m.id}-${outcome}`}
                    className={`br-link is-${outcome} ${m.concluded_at ? "is-settled" : ""}`}
                    d={linkPath(m, to, next.slot, outcome)}
                  />,
                ];
              }),
            )}
          </svg>
          {matches.map((m) => (
            <Card key={m.id} match={m} teams={teams} onStream={m.id === matchId} />
          ))}
        </div>
      </div>
    </OverlayStage>
  );
}

/** From the middle of the source's right edge into the target's slot on its left edge, as the editor draws it. */
function linkPath(
  from: BracketMatch,
  to: BracketMatch,
  slot: 1 | 2,
  outcome: "winner" | "loser",
): string {
  const sx = from.pos_x * UNIT + CARD_W;
  const sy = from.pos_y * UNIT + CARD_H / 2;
  const tx = to.pos_x * UNIT;
  const ty = to.pos_y * UNIT + HEAD_H + ROW_H * (slot - 0.5);
  if (tx - sx >= UNIT) {
    const midX = sx + (tx - sx) * (outcome === "winner" ? 0.5 : 0.75);
    return `M ${sx} ${sy} H ${midX} V ${ty} H ${tx}`;
  }
  const laneY = to.pos_y * UNIT - UNIT / 2;
  return `M ${sx} ${sy} H ${sx + UNIT / 2} V ${laneY} H ${tx - UNIT / 2} V ${ty} H ${tx}`;
}

/** A match's card: its code and time (or "Canlı"), then each team with its banner and score. */
function Card({
  match,
  teams,
  onStream,
}: {
  match: BracketMatch;
  teams: Team[];
  onStream: boolean;
}) {
  const live = isLive(match);
  const winner = winningSlot(match);
  const halves = scoreHalves(match);
  const row = (slot: Side) => {
    const team = teams.find((t) => t.id === (slot === 1 ? match.team1_id : match.team2_id));
    const state = winner === null ? "" : winner === slot ? "is-winner" : "is-loser";
    return (
      <div className={`br-row ${SIDE_CLASS[slot]} ${state}`}>
        <BannerChip team={team} className="br-banner" />
        <span className={`br-name ${team ? "" : "is-empty"}`}>{team?.name ?? "—"}</span>
        <span className="br-score">
          {halves && (live || match.concluded_at) ? halves[slot - 1] : ""}
        </span>
      </div>
    );
  };
  return (
    <div
      className={`br-card ${live ? "is-live" : ""} ${onStream ? "is-on-stream" : ""} ${match.concluded_at ? "is-done" : ""}`}
      style={{ left: match.pos_x * UNIT, top: match.pos_y * UNIT } as CSSProperties}
    >
      <div className="br-head">
        <span className={`br-code is-${match.side}`}>{matchCode(match)}</span>
        <span className="br-when">
          {live ? (
            <>
              <span className="br-live-dot" aria-hidden="true" />
              Canlı
            </>
          ) : match.concluded_at ? (
            (match.stage?.name ?? "Bitti")
          ) : match.starts_at ? (
            formatShortMatchTime(match.starts_at)
          ) : (
            (match.stage?.name ?? "")
          )}
        </span>
      </div>
      {row(1)}
      {row(2)}
    </div>
  );
}

// The spring's stiffness: a critically damped spring with this angular frequency settles in about
// 4.7 / OMEGA seconds (here about two thirds of a second) and never overshoots.
const OMEGA = 7.5;
// Close enough to call it there: a tenth of a pixel, a thousandth of a zoom step.
const REST = 0.1;

/**
 * Glides the board's transform to `target`. Position and zoom (on a log scale, so zooming in and
 * out feel the same) each follow a critically damped spring, which keeps its velocity when the
 * target moves again mid-flight: repeated pushes from a held arrow blend into one motion.
 */
function useSmoothCamera(board: RefObject<HTMLDivElement>, target: Camera, ready: boolean) {
  const goal = useRef(target);
  goal.current = target;
  const state = useRef<{
    x: number;
    y: number;
    z: number;
    vx: number;
    vy: number;
    vz: number;
  } | null>(null);
  const frame = useRef(0);

  useEffect(() => {
    const apply = () => {
      const s = state.current;
      const el = board.current;
      if (!s || !el) return;
      const zoom = Math.exp(s.z);
      el.style.transform = `translate(${VIEW.w / 2 - s.x * zoom}px, ${VIEW.h / 2 - s.y * zoom}px) scale(${zoom})`;
    };
    // Before the bracket arrives there's nothing to look at, and nothing to glide from.
    if (!ready) return;
    if (!state.current) {
      // The first look is where the camera already is.
      state.current = { x: target.x, y: target.y, z: Math.log(target.zoom), vx: 0, vy: 0, vz: 0 };
      apply();
      return;
    }
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let last = performance.now();
    const tick = (now: number) => {
      const s = state.current!;
      const g = goal.current;
      const gz = Math.log(g.zoom);
      if (reduced) {
        Object.assign(s, { x: g.x, y: g.y, z: gz, vx: 0, vy: 0, vz: 0 });
        apply();
        return;
      }
      // Small fixed steps keep the spring stable whatever the frame rate.
      let dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      while (dt > 0) {
        const h = Math.min(dt, 1 / 240);
        dt -= h;
        s.vx += (OMEGA * OMEGA * (g.x - s.x) - 2 * OMEGA * s.vx) * h;
        s.vy += (OMEGA * OMEGA * (g.y - s.y) - 2 * OMEGA * s.vy) * h;
        s.vz += (OMEGA * OMEGA * (gz - s.z) - 2 * OMEGA * s.vz) * h;
        s.x += s.vx * h;
        s.y += s.vy * h;
        s.z += s.vz * h;
      }
      const zoom = Math.exp(s.z);
      const settled =
        Math.abs(g.x - s.x) * zoom < REST &&
        Math.abs(g.y - s.y) * zoom < REST &&
        Math.abs(gz - s.z) < REST / 100 &&
        Math.hypot(s.vx, s.vy) * zoom < REST &&
        Math.abs(s.vz) < REST / 100;
      if (settled) Object.assign(s, { x: g.x, y: g.y, z: gz, vx: 0, vy: 0, vz: 0 });
      apply();
      if (!settled) frame.current = requestAnimationFrame(tick);
    };
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame.current);
  }, [board, ready, target.x, target.y, target.zoom]);
}
