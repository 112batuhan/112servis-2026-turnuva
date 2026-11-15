import type { BracketMatch } from "../../api/types";
import { BOARD } from "../../lib/bracket";

// The bracket page's board and its camera, shared by the overlay (which draws and animates it) and
// the control dock (which moves it), so both agree on where the board is and what fits. Board
// coordinates are the editor's: a match's pos_x/pos_y in board units of BOARD.unit pixels.

export const UNIT = BOARD.unit;
export const CARD_W = BOARD.cardW * UNIT;
export const CARD_H = BOARD.cardH * UNIT;

/** Where the board shows on the 1920×1080 frame, under the page's title. */
export const VIEW = { x: 40, y: 150, w: 1840, h: 890 };

/** What the bracket page looks at: the board point at the view's centre, and how large it's drawn. */
export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export const MIN_ZOOM = 0.2;
export const MAX_ZOOM = 2.4;
export const ZOOM_STEP = 1.25;

export interface Bounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** The board's extent in board pixels; null without matches. */
export function boardBounds(matches: Pick<BracketMatch, "pos_x" | "pos_y">[]): Bounds | null {
  if (matches.length === 0) return null;
  const xs = matches.map((m) => m.pos_x * UNIT);
  const ys = matches.map((m) => m.pos_y * UNIT);
  return {
    left: Math.min(...xs),
    top: Math.min(...ys),
    right: Math.max(...xs) + CARD_W,
    bottom: Math.max(...ys) + CARD_H,
  };
}

const clampZoom = (zoom: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));

/** The whole board in view, with a margin. */
export function fitCamera(bounds: Bounds | null): Camera {
  if (!bounds) return { x: 0, y: 0, zoom: 1 };
  const margin = 2 * UNIT;
  const w = bounds.right - bounds.left + 2 * margin;
  const h = bounds.bottom - bounds.top + 2 * margin;
  return {
    x: (bounds.left + bounds.right) / 2,
    y: (bounds.top + bounds.bottom) / 2,
    zoom: clampZoom(Math.min(VIEW.w / w, VIEW.h / h, 1.4)),
  };
}

/** Keeps the view's centre over the board, so the board never pans out of sight. */
export function clampCamera(camera: Camera, bounds: Bounds | null): Camera {
  const zoom = clampZoom(camera.zoom);
  if (!bounds) return { ...camera, zoom };
  return {
    x: Math.min(bounds.right, Math.max(bounds.left, camera.x)),
    y: Math.min(bounds.bottom, Math.max(bounds.top, camera.y)),
    zoom,
  };
}

/** A match's card centred at a comfortable size. */
export function focusCamera(match: Pick<BracketMatch, "pos_x" | "pos_y">): Camera {
  return { x: match.pos_x * UNIT + CARD_W / 2, y: match.pos_y * UNIT + CARD_H / 2, zoom: 1.6 };
}

/** Moves the camera by a share of the view (dx, dy in -1..1), whatever the zoom. */
export function panCamera(camera: Camera, dx: number, dy: number, bounds: Bounds | null): Camera {
  return clampCamera(
    {
      ...camera,
      x: camera.x + (dx * VIEW.w) / camera.zoom,
      y: camera.y + (dy * VIEW.h) / camera.zoom,
    },
    bounds,
  );
}

export function zoomCamera(camera: Camera, factor: number, bounds: Bounds | null): Camera {
  return clampCamera({ ...camera, zoom: camera.zoom * factor }, bounds);
}
