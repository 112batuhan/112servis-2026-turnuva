import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
} from "react";
import type { BracketMatch, Team } from "../../api/types";
import { BOARD, snapPosition, incomingSources, matchCode, overlapsAny, type Point } from "../../lib/bracket";
import { IconButton } from "../IconButton";
import { BracketMatchCard } from "./BracketMatchCard";
import "./BracketGrid.css";

const UNIT = BOARD.unit;
const CARD_W = BOARD.cardW * UNIT;
const CARD_H = BOARD.cardH * UNIT;
const HEADER_H = 24;
const PAD = UNIT;
// How far the pointer moves before a press becomes a drag (of a card or the view).
const DRAG_THRESHOLD = 4;
const MIN_SCALE = 0.25;
const MAX_SCALE = 2;
// One click of the zoom buttons.
const ZOOM_STEP = 1.25;
// Space left around the matches when recentering.
const FIT_MARGIN = 40;

/** The board's offset in the frame, in screen pixels, and its zoom. */
interface View {
  x: number;
  y: number;
  scale: number;
}

interface Drag {
  id: string;
  /** Where the card was grabbed, relative to its top left corner, in board pixels. */
  grabX: number;
  grabY: number;
  startX: number;
  startY: number;
  pos: Point;
  moved: boolean;
  /** The card would overlap another one here, so dropping it does nothing. */
  blocked: boolean;
}

type Gesture =
  | { kind: "pan"; startX: number; startY: number; view: View; moved: boolean }
  | { kind: "pinch"; distance: number; mid: Point; view: View };

interface Props {
  matches: BracketMatch[];
  teams: Map<string, Team>;
  myTeamId?: string | null;
  /** Marks the matches this user referees or streams. */
  viewerId?: string;
  selectedId?: string | null;
  onSelect?: (id: string | null) => void;
  /** Turns on dragging, keyboard nudging and adding matches. */
  editable?: boolean;
  onMove?: (id: string, x: number, y: number) => void;
  onAdd?: (x: number, y: number) => void;
  /** Makes each card's time a button that opens the calendar on that match. */
  onShowInCalendar?: (id: string) => void;
  /** Shows the time left until each match instead of its date. */
  countdown?: boolean;
  /** Adds a link to each match's referee room. */
  showRefereeRoom?: boolean;
  /** The ids of the matches the search finds, or null when not searching; the rest are dimmed. */
  hits?: Set<string> | null;
}

const clampScale = (s: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s));
// Board units to board pixels.
const px = (x: number) => PAD + x * UNIT;
const py = (y: number) => PAD + y * UNIT;
const distance = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);
const midpoint = (a: Point, b: Point): Point => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });

/**
 * The bracket as cards on a board that pans and zooms: drag empty space to pan, zoom
 * with the mouse wheel, pinch, or the buttons, and recenter to fit every match. In the
 * editor, cards move freely and snap to the board's grid; they can't be dropped onto
 * each other. Arrow keys nudge the focused card (Shift for bigger steps) and
 * double-clicking empty space adds a match there.
 */
export function BracketGrid({
  matches,
  teams,
  myTeamId,
  viewerId,
  selectedId,
  onSelect,
  editable,
  onMove,
  onAdd,
  onShowInCalendar,
  countdown,
  showRefereeRoom,
  hits,
}: Props) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const boardRef = useRef<HTMLDivElement>(null);
  const [view, setView] = useState<View>({ x: 0, y: 0, scale: 1 });
  const viewRef = useRef(view);
  viewRef.current = view;
  const [drag, setDrag] = useState<Drag | null>(null);
  const [hovered, setHovered] = useState<string | null>(null);
  // Every pointer pressed on the frame, by id, in client coordinates.
  const pointers = useRef(new Map<number, Point>());
  const gesture = useRef<Gesture | null>(null);
  const [panning, setPanning] = useState(false);
  // A drag ends with a click on the card or board, which mustn't change the selection.
  const suppressClick = useRef(false);

  const sources = useMemo(() => incomingSources(matches), [matches]);
  const byId = useMemo(() => new Map(matches.map((m) => [m.id, m])), [matches]);

  const posOf = (m: BracketMatch): Point =>
    drag?.moved && drag.id === m.id ? drag.pos : { x: m.pos_x, y: m.pos_y };

  // The board only needs to hold the cards and lines; the frame around it is endless.
  const positions = matches.map(posOf);
  const width = px(Math.max(0, ...positions.map((p) => p.x + BOARD.cardW))) + PAD;
  const height = py(Math.max(0, ...positions.map((p) => p.y + BOARD.cardH))) + PAD;

  // A stage label above the top card of each stage's run of cards in one column and
  // side, like the round names over a bracket.
  const stageLabels = new Map<string, { name: string } & Point>();
  for (const m of matches) {
    if (!m.stage) continue;
    const pos = posOf(m);
    const key = `${m.stage.id}|${m.side}|${pos.x}`;
    const current = stageLabels.get(key);
    if (!current || pos.y < current.y) stageLabels.set(key, { name: m.stage.name, ...pos });
  }

  // Lines touching the selected, hovered or dragged match are drawn on top in the accent colour.
  const focus = drag?.id ?? hovered ?? selectedId ?? null;
  const links = matches.flatMap((from) =>
    (["winner", "loser"] as const).flatMap((outcome) => {
      const next = outcome === "winner" ? from.winner_to : from.loser_to;
      const to = next && byId.get(next.match_id);
      if (!next || !to) return [];
      const active = focus === from.id || focus === to.id;
      const d = linkPath(posOf(from), posOf(to), next.slot, outcome);
      return [{ key: `${from.id}-${outcome}`, d, outcome, active }];
    }),
  );
  links.sort((a, b) => Number(a.active) - Number(b.active));

  // ---------- view ----------

  /** Fits every match in the frame, never zooming in past 100%. */
  const recenter = useCallback(() => {
    const frame = viewportRef.current;
    if (!frame) return;
    const w = frame.clientWidth;
    const h = frame.clientHeight;
    if (matches.length === 0) {
      setView({ x: 0, y: 0, scale: 1 });
      return;
    }
    const minX = px(Math.min(...matches.map((m) => m.pos_x)));
    const minY = py(Math.min(...matches.map((m) => m.pos_y)));
    const maxX = px(Math.max(...matches.map((m) => m.pos_x + BOARD.cardW)));
    const maxY = py(Math.max(...matches.map((m) => m.pos_y + BOARD.cardH)));
    const scale = clampScale(
      Math.min(1, (w - 2 * FIT_MARGIN) / (maxX - minX), (h - 2 * FIT_MARGIN) / (maxY - minY)),
    );
    setView({ scale, x: w / 2 - ((minX + maxX) / 2) * scale, y: h / 2 - ((minY + maxY) / 2) * scale });
  }, [matches]);

  // Opens recentered once there's something to show.
  const fitted = useRef(false);
  useLayoutEffect(() => {
    if (fitted.current || matches.length === 0) return;
    fitted.current = true;
    recenter();
  }, [matches, recenter]);

  /** Zooms by `factor`, keeping the point at (cx, cy) in the frame where it is. */
  const zoomAt = useCallback((factor: number, cx: number, cy: number) => {
    setView((v) => {
      const scale = clampScale(v.scale * factor);
      const k = scale / v.scale;
      return { scale, x: cx - (cx - v.x) * k, y: cy - (cy - v.y) * k };
    });
  }, []);

  /** Pans just enough to bring a card fully into the frame, e.g. when it gets keyboard focus. */
  function reveal(m: BracketMatch) {
    const frame = viewportRef.current!;
    setView((v) => {
      const left = v.x + px(m.pos_x) * v.scale;
      const top = v.y + py(m.pos_y) * v.scale;
      const right = left + CARD_W * v.scale;
      const bottom = top + CARD_H * v.scale;
      const dx = left < 0 ? -left + PAD : right > frame.clientWidth ? frame.clientWidth - right - PAD : 0;
      const dy = top < 0 ? -top + PAD : bottom > frame.clientHeight ? frame.clientHeight - bottom - PAD : 0;
      return dx || dy ? { ...v, x: v.x + dx, y: v.y + dy } : v;
    });
  }

  function zoomCentered(factor: number) {
    const frame = viewportRef.current!;
    zoomAt(factor, frame.clientWidth / 2, frame.clientHeight / 2);
  }

  // React's wheel listeners are passive, so the page would scroll along; this one isn't.
  useEffect(() => {
    const frame = viewportRef.current!;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      // Lines (Firefox) to roughly pixels; capped so one notch never jumps too far.
      const delta = Math.max(-100, Math.min(100, e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY));
      const rect = frame.getBoundingClientRect();
      zoomAt(Math.exp(-delta * 0.002), e.clientX - rect.left, e.clientY - rect.top);
    };
    frame.addEventListener("wheel", onWheel, { passive: false });
    return () => frame.removeEventListener("wheel", onWheel);
  }, [zoomAt]);

  /** A client point on the board, in board pixels. */
  function boardPoint(clientX: number, clientY: number): Point {
    const rect = viewportRef.current!.getBoundingClientRect();
    const v = viewRef.current;
    return { x: (clientX - rect.left - v.x) / v.scale, y: (clientY - rect.top - v.y) / v.scale };
  }

  /** The snapped position of a card held at (grabX, grabY) under the pointer. */
  function cardPosition(clientX: number, clientY: number, grabX: number, grabY: number): Point {
    const p = boardPoint(clientX, clientY);
    return { x: snapPosition((p.x - grabX - PAD) / UNIT), y: snapPosition((p.y - grabY - PAD) / UNIT) };
  }

  // ---------- panning and pinching ----------

  const isBackground = (target: EventTarget) => target === viewportRef.current || target === boardRef.current;

  function startPinch() {
    const [a, b] = [...pointers.current.values()];
    const frame = viewportRef.current!;
    // Both fingers report to the frame from now on, even one that started on a card.
    for (const id of pointers.current.keys()) {
      try {
        frame.setPointerCapture(id);
      } catch {
        // The pointer is already gone.
      }
    }
    gesture.current = { kind: "pinch", distance: distance(a, b), mid: midpoint(a, b), view: viewRef.current };
    setDrag(null);
    setPanning(true);
  }

  const frameProps = {
    onPointerDown: (e: PointerEvent<HTMLDivElement>) => {
      pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (pointers.current.size === 2) {
        startPinch();
        return;
      }
      if (pointers.current.size > 2 || !isBackground(e.target) || e.button !== 0) return;
      // Keeps the press from selecting text as it moves.
      e.preventDefault();
      e.currentTarget.setPointerCapture(e.pointerId);
      gesture.current = { kind: "pan", startX: e.clientX, startY: e.clientY, view: viewRef.current, moved: false };
    },
    onPointerMove: (e: PointerEvent<HTMLDivElement>) => {
      if (!pointers.current.has(e.pointerId)) return;
      pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const g = gesture.current;
      if (g?.kind === "pinch" && pointers.current.size >= 2) {
        const [a, b] = [...pointers.current.values()];
        const rect = e.currentTarget.getBoundingClientRect();
        const scale = clampScale((g.view.scale * distance(a, b)) / g.distance);
        // The board point that was under the fingers' midpoint stays under it.
        const bx = (g.mid.x - rect.left - g.view.x) / g.view.scale;
        const by = (g.mid.y - rect.top - g.view.y) / g.view.scale;
        const mid = midpoint(a, b);
        setView({ scale, x: mid.x - rect.left - bx * scale, y: mid.y - rect.top - by * scale });
      } else if (g?.kind === "pan") {
        const dx = e.clientX - g.startX;
        const dy = e.clientY - g.startY;
        if (!g.moved) {
          if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
          g.moved = true;
          setPanning(true);
        }
        setView({ ...g.view, x: g.view.x + dx, y: g.view.y + dy });
      }
    },
    onPointerUp: (e: PointerEvent<HTMLDivElement>) => endPointer(e.pointerId),
    onPointerCancel: (e: PointerEvent<HTMLDivElement>) => endPointer(e.pointerId),
  };

  function endPointer(pointerId: number) {
    pointers.current.delete(pointerId);
    const g = gesture.current;
    if (!g) return;
    if (g.kind === "pan" && g.moved) suppressClick.current = true;
    // Lifting one finger of a pinch ends it; the other one doesn't start a pan.
    if (g.kind === "pinch") suppressClick.current = true;
    if (g.kind === "pan" || pointers.current.size < 2) {
      gesture.current = null;
      setPanning(false);
    }
  }

  // ---------- editing ----------

  function nudge(e: KeyboardEvent, m: BracketMatch) {
    const step = e.shiftKey ? 5 : 1;
    const delta: Record<string, Point> = {
      ArrowLeft: { x: -step, y: 0 },
      ArrowRight: { x: step, y: 0 },
      ArrowUp: { x: 0, y: -step },
      ArrowDown: { x: 0, y: step },
    };
    const d = delta[e.key];
    if (!d) return;
    e.preventDefault();
    const next = { x: snapPosition(m.pos_x + d.x), y: snapPosition(m.pos_y + d.y) };
    if ((next.x !== m.pos_x || next.y !== m.pos_y) && !overlapsAny(matches, next, m.id)) {
      onMove?.(m.id, next.x, next.y);
    }
  }

  function addAt(e: MouseEvent<HTMLDivElement>) {
    if (!isBackground(e.target) || !onAdd) return;
    // Centred on the pointer.
    const pos = cardPosition(e.clientX, e.clientY, CARD_W / 2, CARD_H / 2);
    if (!overlapsAny(matches, pos)) onAdd(pos.x, pos.y);
  }

  const frameStyle = {
    // The editor's dots follow the board as it pans and zooms.
    "--grid-size": `${UNIT * view.scale}px`,
    "--grid-x": `${view.x + (PAD - UNIT / 2) * view.scale}px`,
    "--grid-y": `${view.y + (PAD - UNIT / 2) * view.scale}px`,
  } as CSSProperties;

  const boardStyle = {
    width,
    height,
    transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})`,
    "--bracket-card-w": `${CARD_W}px`,
    "--bracket-card-h": `${CARD_H}px`,
    "--bracket-header-h": `${HEADER_H}px`,
    "--bracket-unit": `${UNIT}px`,
  } as CSSProperties;

  const frameClasses = ["bracket-viewport", editable && "is-editable", panning && "is-panning"]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      ref={viewportRef}
      className={frameClasses}
      style={frameStyle}
      {...frameProps}
      onClick={(e) => {
        if (!isBackground(e.target)) return;
        if (suppressClick.current) {
          suppressClick.current = false;
          return;
        }
        onSelect?.(null);
      }}
      onDoubleClick={editable ? addAt : undefined}
      // Focusing an off-screen card scrolls even a frame without scrollbars, which the
      // view doesn't account for; undo it (onFocus pans to the card instead).
      onScroll={(e) => {
        e.currentTarget.scrollLeft = 0;
        e.currentTarget.scrollTop = 0;
      }}
    >
      <div ref={boardRef} className="bracket-board" style={boardStyle}>
        <svg className="bracket-lines" width={width} height={height} aria-hidden="true">
          {links.map((link) => (
            <path
              key={link.key}
              d={link.d}
              className={`bracket-line is-${link.outcome} ${link.active ? "is-active" : ""}`}
            />
          ))}
        </svg>

        {[...stageLabels].map(([key, label]) => (
          <div
            key={key}
            className="bracket-stage-label"
            style={{ left: px(label.x), top: py(label.y) - UNIT }}
            aria-hidden="true"
          >
            {label.name}
          </div>
        ))}

        {matches.map((m) => {
          const pos = posOf(m);
          const dragging = drag?.id === m.id && drag.moved;
          const mine = viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId);
          const code = m.stage ? `${matchCode(m)} (${m.stage.name})` : matchCode(m);
          const classes = [
            "bracket-slot",
            m.id === selectedId && "is-selected",
            mine && "is-mine",
            dragging && "is-dragging",
            dragging && drag.blocked && "is-blocked",
            onSelect && "is-clickable",
            hits && !hits.has(m.id) && "is-dimmed",
          ]
            .filter(Boolean)
            .join(" ");
          return (
            <div
              key={m.id}
              className={classes}
              style={{ left: px(pos.x), top: py(pos.y) }}
              role={onSelect ? "button" : undefined}
              tabIndex={onSelect ? 0 : undefined}
              aria-pressed={onSelect ? m.id === selectedId : undefined}
              aria-label={
                onSelect ? `${code} maçını seç${editable ? " (ok tuşlarıyla taşı)" : ""}` : undefined
              }
              onPointerDown={(e) => {
                // A second finger is a pinch, not a card drag.
                if (!editable || e.button !== 0 || pointers.current.size > 0) return;
                const rect = e.currentTarget.getBoundingClientRect();
                const scale = viewRef.current.scale;
                setDrag({
                  id: m.id,
                  grabX: (e.clientX - rect.left) / scale,
                  grabY: (e.clientY - rect.top) / scale,
                  startX: e.clientX,
                  startY: e.clientY,
                  pos: { x: m.pos_x, y: m.pos_y },
                  moved: false,
                  blocked: false,
                });
              }}
              onPointerMove={(e) => {
                if (drag?.id !== m.id) return;
                // Released off the card before it was captured, so its pointerup never came.
                if (e.buttons === 0) {
                  setDrag(null);
                  return;
                }
                if (!drag.moved) {
                  if (Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) < DRAG_THRESHOLD) return;
                  // Captured only now, so a plain click still reaches what's under it (a team name).
                  e.currentTarget.setPointerCapture(e.pointerId);
                }
                const next = cardPosition(e.clientX, e.clientY, drag.grabX, drag.grabY);
                if (drag.moved && next.x === drag.pos.x && next.y === drag.pos.y) return;
                setDrag({ ...drag, pos: next, moved: true, blocked: overlapsAny(matches, next, m.id) });
              }}
              onPointerUp={() => {
                if (drag?.id !== m.id) return;
                if (drag.moved) {
                  suppressClick.current = true;
                  const changed = drag.pos.x !== m.pos_x || drag.pos.y !== m.pos_y;
                  if (changed && !drag.blocked) onMove?.(m.id, drag.pos.x, drag.pos.y);
                }
                setDrag(null);
              }}
              onPointerCancel={() => setDrag(null)}
              onFocus={(e) => {
                // Only keyboard focus; a click on a half-visible card mustn't move the view under it.
                if (e.currentTarget.matches(":focus-visible")) reveal(m);
              }}
              onMouseEnter={() => setHovered(m.id)}
              onMouseLeave={() => setHovered(null)}
              onClick={() => {
                if (suppressClick.current) {
                  suppressClick.current = false;
                  return;
                }
                onSelect?.(m.id === selectedId ? null : m.id);
              }}
              onKeyDown={(e) => {
                if (onSelect && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  onSelect(m.id === selectedId ? null : m.id);
                } else if (editable) {
                  nudge(e, m);
                }
              }}
            >
              <BracketMatchCard
                match={m}
                teams={teams}
                sources={sources.get(m.id) ?? [null, null]}
                myTeamId={myTeamId}
                onShowInCalendar={onShowInCalendar}
                countdown={countdown}
                showRefereeRoom={showRefereeRoom}
              />
            </div>
          );
        })}
      </div>

      <div className="bracket-controls">
        <IconButton icon="minus" label="Uzaklaştır" onClick={() => zoomCentered(1 / ZOOM_STEP)} />
        <button
          type="button"
          className="bracket-zoom-level"
          title="%100'e dön"
          aria-label={`Yakınlaştırma %${Math.round(view.scale * 100)}, %100'e dön`}
          onClick={() => zoomCentered(1 / view.scale)}
        >
          %{Math.round(view.scale * 100)}
        </button>
        <IconButton icon="plus" label="Yakınlaştır" onClick={() => zoomCentered(ZOOM_STEP)} />
        <IconButton icon="recenter" label="Ortala" onClick={recenter} />
      </div>
    </div>
  );
}

// From the middle of the source's right edge into the target's slot on its left edge.
// Loser lines turn closer to the target so they don't run along the winner lines. A
// target that isn't to the right is reached through the gap above it.
function linkPath(from: Point, to: Point, slot: 1 | 2, outcome: "winner" | "loser"): string {
  const sx = PAD + from.x * UNIT + CARD_W;
  const sy = PAD + from.y * UNIT + CARD_H / 2;
  const tx = PAD + to.x * UNIT;
  const ty = PAD + to.y * UNIT + HEADER_H + ((CARD_H - HEADER_H) / 2) * (slot - 0.5);
  if (tx - sx >= UNIT) {
    const midX = sx + (tx - sx) * (outcome === "winner" ? 0.5 : 0.75);
    return `M ${sx} ${sy} H ${midX} V ${ty} H ${tx}`;
  }
  const outX = sx + UNIT / 2;
  const inX = tx - UNIT / 2;
  const laneY = PAD + to.y * UNIT - UNIT / 2;
  return `M ${sx} ${sy} H ${outX} V ${laneY} H ${inX} V ${ty} H ${tx}`;
}
